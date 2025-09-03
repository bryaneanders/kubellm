use crate::KeywordChecker;
use linked_hash_map::LinkedHashMap;

const STANDARD_CODE_BLOCK_TEXT_COLOR_ESC: &str = "\x1b[39m";
const QUOTED_CODE_BLOCK_TEXT_COLOR_ESC: &str = "\x1b[32m";
const START_CODE_BLOCK_SECTION_ESC: &str = "\x1b[39m\x1b[40m";
const START_COMMENT_SECTION_ESC: &str = "\x1b[38;5;92m\x1b[40m";
const END_CODE_BLOCK_SECTION_ESC: &str = "\x1b[97m\x1b[49m";
const SYNTAX_HIGHLIGHTING_ESC: &str = "\x1b[38;5;215m";
const NON_BOLD_TEXT_ESC: &str = "\x1b[22;24m";
const BOLD_TEXT_ESC: &str = "\x1b[1;4m";

#[derive(Debug)]
pub struct PromptFormatter {
    bold_section: bool,
    code_block_section: bool,
    single_line_comment_section: bool,
    multi_line_comment_section: bool,
    code_block_double_quote_section: bool,
    code_block_single_quote_section: bool,
    code_block_strings: LinkedHashMap<String, String>,
    longest_code_block_string: usize,
}

impl Default for PromptFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptFormatter {
    pub fn new() -> Self {
        Self {
            bold_section: false,
            code_block_section: false,
            multi_line_comment_section: false,
            single_line_comment_section: false,
            code_block_double_quote_section: false,
            code_block_single_quote_section: false,
            code_block_strings: LinkedHashMap::new(),
            longest_code_block_string: 0,
        }
    }

    /// Takes string and formats it to wrap at a width and format it for emphasis and markdown code blocks
    pub fn format_prompt(&mut self, text: &str, width: usize) -> Vec<String> {
        let mut formatted_prompt = Vec::new();
        let mut language: String = "".to_owned();

        for paragraph in text.split('\n') {
            if paragraph.trim().is_empty() {
                let mut current_line: String = String::new();
                if self.code_block_section {
                    current_line.push_str(START_CODE_BLOCK_SECTION_ESC);
                    self.code_block_strings.insert("".to_string(), current_line);
                } else {
                    formatted_prompt.push(current_line); // Preserve empty lines
                }
                continue;
            }

            let indent_prefix = if paragraph.starts_with("-") { " " } else { "" };

            let leading_whitespace = paragraph.len() - paragraph.trim_start().len();
            let unformatted_indent = paragraph[..leading_whitespace].to_string();
            let mut indent = unformatted_indent.clone();
            // need this so that escape characters don't count towards the length of the line
            let mut unformatted_line = unformatted_indent.to_owned();
            if self.code_block_section {
                indent.insert_str(0, START_CODE_BLOCK_SECTION_ESC);
            }

            let mut current_line = indent.to_owned();

            self.single_line_comment_section = false;
            for word in paragraph.split_whitespace() {
                // handle line wrap
                if unformatted_line.len() + word.len() + 1 > width && !unformatted_line.is_empty() {
                    if self.code_block_section {
                        if unformatted_line.len() > self.longest_code_block_string {
                            self.longest_code_block_string = unformatted_line.len();
                        }

                        self.code_block_strings
                            .insert(unformatted_line, current_line);
                    } else {
                        formatted_prompt.push(current_line);
                    }

                    current_line = indent.to_owned();
                    unformatted_line = unformatted_indent.to_owned();

                    self.handle_code_block_line_wrap(&mut current_line, &mut unformatted_line);
                    current_line.push_str(indent_prefix);
                }

                if !current_line.is_empty() {
                    current_line.push(' ');
                    unformatted_line.push(' ');
                }

                let mut processed_word = word.to_owned();
                // don't handle bold in code block
                if processed_word.contains("**") && !self.code_block_section {
                    self.handle_bold_formatting(&mut processed_word);
                }

                if processed_word.contains("```") {
                    self.handle_code_block_formatting(
                        &mut processed_word,
                        &mut language,
                        width,
                        &mut formatted_prompt,
                    );
                    self.bold_section = false;
                    processed_word.insert_str(processed_word.len(), NON_BOLD_TEXT_ESC);
                    if self.code_block_section {
                        current_line = "".to_owned();
                        unformatted_line = "".to_owned();
                        formatted_prompt.push("\x1b[1A".to_owned());
                        break;
                    }
                }

                // after here its just formatting, not the word itself
                unformatted_line.push_str(&processed_word);

                // handle comment flags
                self.handle_comment_flags(&mut current_line, &mut processed_word);

                if self.code_block_section
                    && !self.single_line_comment_section
                    && !self.multi_line_comment_section
                {
                    self.handle_syntax_highlighting(&mut processed_word, &language)
                }

                current_line.push_str(&processed_word);
            }

            if !current_line.trim().is_empty() {
                if self.code_block_section {
                    if unformatted_line.len() > self.longest_code_block_string {
                        self.longest_code_block_string = unformatted_line.len();
                    }
                    self.code_block_strings
                        .insert(unformatted_line, current_line);
                } else {
                    formatted_prompt.push(current_line);
                }
            }
        }

        formatted_prompt
    }

    fn handle_code_block_line_wrap(
        &mut self,
        current_line: &mut String,
        unformatted_line: &mut String,
    ) {
        if self.code_block_section {
            if !(self.single_line_comment_section || self.multi_line_comment_section)
                && !(self.code_block_single_quote_section || self.code_block_double_quote_section)
            {
                current_line.push_str(START_CODE_BLOCK_SECTION_ESC);
            } else if self.single_line_comment_section || self.multi_line_comment_section {
                current_line.push_str(START_COMMENT_SECTION_ESC);
            } else if self.code_block_single_quote_section || self.code_block_double_quote_section {
                current_line.push_str(QUOTED_CODE_BLOCK_TEXT_COLOR_ESC);
            }

            current_line.push_str("    ");
            unformatted_line.push_str("    ");
        }
    }

    /// Process comment formatting
    fn handle_comment_flags(&mut self, current_line: &mut String, processed_word: &mut String) {
        if processed_word.contains("//")
            && self.code_block_section
            && !self.multi_line_comment_section
        {
            self.single_line_comment_section = true;
            current_line.insert_str(current_line.len(), START_COMMENT_SECTION_ESC);
        } else if processed_word.contains("/*")
            && self.code_block_section
            && !self.single_line_comment_section
            && !self.multi_line_comment_section
        {
            self.multi_line_comment_section = true;
            current_line.insert_str(current_line.len(), START_COMMENT_SECTION_ESC);
        } else if processed_word.contains("*/")
            && self.code_block_section
            && self.multi_line_comment_section
        {
            self.multi_line_comment_section = false;
            processed_word.insert_str(processed_word.len(), START_CODE_BLOCK_SECTION_ESC);
        }
    }

    /// Turn on and off emphasized formatting (bold and underlined)
    fn handle_bold_formatting(&mut self, processed_word: &mut String) {
        while processed_word.contains("**") {
            *processed_word = if self.bold_section {
                self.bold_section = false;
                // bold and underlined
                processed_word.replacen("**", NON_BOLD_TEXT_ESC, 1)
            } else {
                self.bold_section = true;
                // back to unbold and non-underlined
                processed_word.replacen("**", BOLD_TEXT_ESC, 1)
            }
        }
    }

    /// Turns on and off code block formatting
    fn handle_code_block_formatting(
        &mut self,
        processed_word: &mut String,
        language: &mut String,
        width: usize,
        formatted_prompt: &mut Vec<String>,
    ) {
        if !self.code_block_section {
            self.code_block_section = true;
            self.code_block_strings = LinkedHashMap::new();
            self.longest_code_block_string = width;
            *language = processed_word.replace("```", "").replace("[.*", "");
            *processed_word = format!("\x1b[1A\x1b[2K{}", END_CODE_BLOCK_SECTION_ESC)
        } else {
            self.code_block_section = false;
            // pad all lines to the longest code block line length if longer than width
            let unpadded_lines: Vec<_> = self.code_block_strings.keys().cloned().collect();
            for unformatted_line in unpadded_lines {
                let current_len = if unformatted_line.len() < width {
                    unformatted_line.len()
                } else {
                    width
                };
                match self.code_block_strings.get_mut(&unformatted_line) {
                    formatted_line => {
                        let mut formatted_line = formatted_line.unwrap();
                        pad_code_block_line(
                            &mut formatted_line,
                            self.longest_code_block_string - current_len,
                        );
                        formatted_line.push_str(END_CODE_BLOCK_SECTION_ESC);
                    }
                }
            }
            formatted_prompt.extend(self.code_block_strings.values().cloned());

            *language = "".to_owned();
            *processed_word = processed_word.replace(
                "```",
                format!("\r{}\x1b[K  |", END_CODE_BLOCK_SECTION_ESC).as_str(),
            );
        }
    }

    /// Handles text color changes for different syntax highlighting situations
    fn handle_syntax_highlighting(&mut self, processed_word: &mut String, language: &str) {
        // only currently implemented at all for rust, java, bash
        if self.code_block_single_quote_section || !self.code_block_double_quote_section {
            if let Ok(is_keyword) = KeywordChecker::is_keyword(processed_word, language) {
                if is_keyword {
                    if processed_word.contains("for") {
                        println!("Processed word containing for: {}", processed_word);
                    }
                    processed_word.insert_str(0, SYNTAX_HIGHLIGHTING_ESC);
                    processed_word
                        .insert_str(processed_word.len(), STANDARD_CODE_BLOCK_TEXT_COLOR_ESC);
                    return;
                }
            }
        }

        // quote highlighting
        if processed_word.contains("\"") || processed_word.contains("'") {
            let mut edited_word = processed_word.clone();
            let mut offset = 0;

            for (i, c) in processed_word.chars().enumerate() {
                if ((c == '"' && !self.code_block_single_quote_section)
                    || (c == '\'' && !self.code_block_double_quote_section))
                    && (i == 0 || processed_word.chars().nth(i - 1).unwrap() != '\\')
                {
                    if self.code_block_single_quote_section || self.code_block_double_quote_section
                    {
                        if c == '\'' {
                            self.code_block_single_quote_section = false;
                        } else {
                            self.code_block_double_quote_section = false;
                        }
                        edited_word.insert_str(i + offset + 1, STANDARD_CODE_BLOCK_TEXT_COLOR_ESC);
                        offset += STANDARD_CODE_BLOCK_TEXT_COLOR_ESC.len();
                    } else {
                        if c == '\'' {
                            self.code_block_single_quote_section = true;
                        } else {
                            self.code_block_double_quote_section = true;
                        }
                        edited_word.insert_str(i + offset, QUOTED_CODE_BLOCK_TEXT_COLOR_ESC);
                        offset += QUOTED_CODE_BLOCK_TEXT_COLOR_ESC.len();
                    }
                }
            }
            *processed_word = edited_word;
        }
    }
}

/// Pad the end of code block lines to the width to maintain a constant appearance
fn pad_code_block_line(formatted_line: &mut String, pad_length: usize) {
    if pad_length > 0 {
        formatted_line.push_str(START_CODE_BLOCK_SECTION_ESC);
        formatted_line.insert_str(formatted_line.len(), &" ".repeat(pad_length));
        formatted_line.push_str(END_CODE_BLOCK_SECTION_ESC);
    }
}
