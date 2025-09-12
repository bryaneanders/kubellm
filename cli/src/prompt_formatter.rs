use crate::KeywordChecker;

const STANDARD_CODE_BLOCK_TEXT_COLOR_ESC: &str = "\x1b[39m";
const QUOTED_CODE_BLOCK_TEXT_COLOR_ESC: &str = "\x1b[32m";
const START_CODE_BLOCK_SECTION_ESC: &str = "\x1b[39m\x1b[40m";
const START_COMMENT_SECTION_ESC: &str = "\x1b[38;5;92m\x1b[40m";
const END_CODE_BLOCK_SECTION_ESC: &str = "\x1b[97m\x1b[49m";
const SYNTAX_HIGHLIGHTING_ESC: &str = "\x1b[38;5;215m";
const NON_BOLD_TEXT_ESC: &str = "\x1b[22;24m";
const BOLD_TEXT_ESC: &str = "\x1b[1;4m";
const DEFAULT_WIDTH: usize = 80;
const MAX_WIDTH: usize = 121;

#[derive(Debug)]
pub struct PromptFormatter {
    formatted_prompt: Vec<String>,
    bold_section: bool,
    code_block_section: bool,
    first_code_block_line: bool,
    language: String,
    single_line_comment_section: bool,
    multi_line_comment_section: bool,
    code_block_double_quote_section: bool,
    code_block_single_quote_section: bool,
    width: usize,
    code_block_width: usize,
}

impl Default for PromptFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptFormatter {
    pub fn new() -> Self {
        Self {
            formatted_prompt: Vec::new(),
            bold_section: false,
            code_block_section: false,
            first_code_block_line: false,
            language: "".to_owned(),
            multi_line_comment_section: false,
            single_line_comment_section: false,
            code_block_double_quote_section: false,
            code_block_single_quote_section: false,
            width: DEFAULT_WIDTH,
            code_block_width: DEFAULT_WIDTH,
        }
    }

    /// Takes string and formats it to wrap at a width and format it for emphasis and markdown code blocks
    pub fn format_prompt(&mut self, text: &str, width: usize) -> &Vec<String> {
        if !self.formatted_prompt.is_empty() {
            self.formatted_prompt = Vec::new();
        }
        self.width = width;
        self.code_block_width = width;
        self.determine_max_width(text);

        for paragraph in text.split('\n') {
            // empty line
            if paragraph.trim().is_empty() {
                self.add_formatted_line(0, String::new());
                continue;
            }

            // get the level of indent to preserve for code blocks.
            let indent_prefix = if paragraph.starts_with("-") { " " } else { "" };
            let leading_whitespace = paragraph.len() - paragraph.trim_start().len();
            let unformatted_indent = &paragraph[..leading_whitespace];

            let mut indent = String::with_capacity(
                unformatted_indent.len() + START_CODE_BLOCK_SECTION_ESC.len(),
            );
            if self.code_block_section {
                indent.push_str(START_CODE_BLOCK_SECTION_ESC);
            }
            indent.push_str(unformatted_indent);

            let mut current_line = String::with_capacity(self.width);
            current_line.push_str(&indent);
            // need this so that escape characters don't count towards the length of the line
            let mut unformatted_line = String::with_capacity(self.width);
            unformatted_line.push_str(unformatted_indent);

            self.single_line_comment_section = false;
            for word in paragraph.split_whitespace() {
                // handle line wrap
                let width_to_use = if self.code_block_section {
                    self.code_block_width
                } else {
                    self.width
                };

                // width of line + 2 for space between words and 1 for the end quote
                let mut line_len = unformatted_line.len() + word.len() + 2;
                if self.code_block_single_quote_section || self.code_block_double_quote_section {
                    line_len += 1;
                }

                // wrap the line if the next word will make the line longer than the width
                if line_len > width_to_use && !unformatted_line.is_empty() {
                    // add closing quotes to end of line
                    if self.code_block_single_quote_section || self.code_block_double_quote_section
                    {
                        current_line.push('"');
                        unformatted_line.push('"');
                    }
                    self.add_formatted_line(unformatted_line.len(), current_line);

                    // start new lines with the same level of indent
                    current_line = String::with_capacity(width_to_use);
                    current_line.push_str(&indent);
                    unformatted_line = String::with_capacity(width_to_use);
                    unformatted_line.push_str(unformatted_indent);

                    self.handle_code_block_line_wrap(&mut current_line, &mut unformatted_line);
                    current_line.push_str(indent_prefix);

                    if self.code_block_single_quote_section || self.code_block_double_quote_section
                    {
                        current_line.push_str(START_CODE_BLOCK_SECTION_ESC);
                        current_line.push('+');
                        current_line.push_str(QUOTED_CODE_BLOCK_TEXT_COLOR_ESC);
                        current_line.push_str(" \"");
                        unformatted_line.push_str("+ \"");
                    }
                }

                // add space between words
                if !current_line.is_empty() {
                    current_line.push(' ');
                    unformatted_line.push(' ');
                }

                let mut processed_word = word.to_owned();
                // handle bold
                if processed_word.contains("**") && !self.code_block_section {
                    self.handle_bold_formatting(&mut processed_word);
                }

                //  code block formatting
                if processed_word.contains("```") {
                    self.handle_code_block_formatting(&mut processed_word);
                    self.bold_section = false;
                    processed_word.insert_str(processed_word.len(), NON_BOLD_TEXT_ESC);
                    if self.code_block_section {
                        current_line.clear();
                        unformatted_line.clear();
                        //self.formatted_prompt.push("\x1b[1A".to_owned());
                        break;
                    }
                }

                unformatted_line.push_str(&processed_word);

                // handle comment flags
                self.handle_comment_flags(&mut current_line, &mut processed_word);

                // syntax highlighting when in a code block but not in a comment
                if self.code_block_section
                    && !self.single_line_comment_section
                    && !self.multi_line_comment_section
                {
                    self.handle_syntax_highlighting(&mut processed_word)
                }

                current_line.push_str(&processed_word);
            }

            // don't print a code block formatted line if its the line where we replaced
            // ```lang with nothing but formatting
            if self.first_code_block_line {
                self.first_code_block_line = false;
            } else {
                self.add_formatted_line(unformatted_line.len(), current_line);
            }
        }

        &self.formatted_prompt
    }

    fn add_formatted_line(&mut self, unformatted_line_len: usize, mut current_line: String) {
        if self.code_block_section {
            current_line.push_str(START_CODE_BLOCK_SECTION_ESC);
            self.pad_code_block_line(&mut current_line, unformatted_line_len);
        }
        self.formatted_prompt.push(current_line);
    }

    fn handle_code_block_line_wrap(
        &mut self,
        current_line: &mut String,
        unformatted_line: &mut String,
    ) {
        if self.code_block_section {
            if !(self.single_line_comment_section
                || self.multi_line_comment_section
                || self.code_block_single_quote_section
                || self.code_block_double_quote_section)
            {
                current_line.push_str(START_CODE_BLOCK_SECTION_ESC);
            } else if self.single_line_comment_section || self.multi_line_comment_section {
                current_line.push_str(START_COMMENT_SECTION_ESC);
            } else if self.code_block_single_quote_section || self.code_block_double_quote_section {
                current_line.push_str(QUOTED_CODE_BLOCK_TEXT_COLOR_ESC);
            }

            // Indent 4 char "tab"
            // todo should this actually be a tab instead of spaces?
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
    fn handle_code_block_formatting(&mut self, processed_word: &mut String) {
        if !self.code_block_section {
            self.code_block_section = true;
            self.first_code_block_line = true;
            self.language = processed_word.replace("```", "").replace("[.*", "");
        } else {
            self.code_block_section = false;
            self.language = "".to_owned();
            *processed_word = processed_word.replace(
                "```",
                format!("\r{}\x1b[K  |", END_CODE_BLOCK_SECTION_ESC).as_str(),
            );
        }
    }

    /// Handles text color changes for different syntax highlighting situations
    fn handle_syntax_highlighting(&mut self, processed_word: &mut String) {
        // only currently implemented at all for rust, java, bash
        if self.code_block_single_quote_section || !self.code_block_double_quote_section {
            if let Ok(is_keyword) = KeywordChecker::is_keyword(processed_word, &self.language) {
                if is_keyword {
                    processed_word.insert_str(0, SYNTAX_HIGHLIGHTING_ESC);
                    processed_word
                        .insert_str(processed_word.len(), STANDARD_CODE_BLOCK_TEXT_COLOR_ESC);
                    return;
                }
            }
        }

        self.handle_quote_formatting(processed_word, false);
    }

    /// Handles text color changes for different quote syntax highlighting situations
    fn handle_quote_formatting(&mut self, processed_word: &mut String, finding_width: bool) {
        if !(processed_word.contains("\"") || processed_word.contains("'")) {
            return;
        }

        // Only allocate if we need to modify the string for formatting
        if finding_width {
            // When finding width, we only need to update state, no string modifications
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
                    } else if c == '\'' {
                        self.code_block_single_quote_section = true;
                    } else {
                        self.code_block_double_quote_section = true;
                    }
                }
            }
            return;
        }

        // For formatting, we need to modify the string - build it efficiently
        let original_chars: Vec<char> = processed_word.chars().collect();
        let mut result = String::with_capacity(processed_word.len() + 50); // estimate extra space for escape codes

        for (i, &c) in original_chars.iter().enumerate() {
            // handle quote syntax highlighting if a quote character, but not an escaped one
            if ((c == '"' && !self.code_block_single_quote_section)
                || (c == '\'' && !self.code_block_double_quote_section))
                && (i == 0 || original_chars[i - 1] != '\\')
            {
                if self.code_block_single_quote_section || self.code_block_double_quote_section {
                    if c == '\'' {
                        self.code_block_single_quote_section = false;
                    } else {
                        self.code_block_double_quote_section = false;
                    }
                    result.push(c);
                    result.push_str(STANDARD_CODE_BLOCK_TEXT_COLOR_ESC);
                } else {
                    if c == '\'' {
                        self.code_block_single_quote_section = true;
                    } else {
                        self.code_block_double_quote_section = true;
                    }
                    result.push_str(QUOTED_CODE_BLOCK_TEXT_COLOR_ESC);
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        *processed_word = result;
    }

    /// Determine the max width of code blocks based on the length of word wraps
    pub fn determine_max_width(&mut self, text: &str) {
        for paragraph in text.split('\n') {
            if paragraph.trim().is_empty() {
                continue;
            }

            let leading_whitespace = paragraph.len() - paragraph.trim_start().len();
            let indent = &paragraph[..leading_whitespace];
            let mut current_line = String::with_capacity(self.width);
            current_line.push_str(indent);

            for word in paragraph.split_whitespace() {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }

                // replace chars that are removed during formatting
                let word = word.replace("**", "").replace("```", "");
                current_line.push_str(&word);

                // 2 is for space between string and word and 1 pad char at the end of the line
                let next_word_str_len = current_line.len() + word.len() + 2;
                let mut one_word_str_len = indent.len() + word.len() + 2;
                if current_line.contains(" ") {
                    one_word_str_len += 4; // extra indent
                }

                let mut word = word.to_string();
                self.handle_quote_formatting(&mut word, true);
                if self.code_block_single_quote_section || self.code_block_double_quote_section {
                    one_word_str_len += 1; // end quotes
                }

                // if indent + word > current width but < MAX_WIDTH increase width
                if one_word_str_len > self.code_block_width && one_word_str_len < MAX_WIDTH {
                    self.code_block_width = one_word_str_len;
                    current_line.clear();
                    current_line.push_str(indent);
                } else if one_word_str_len >= MAX_WIDTH {
                    // if indent + word >= max width use max width
                    self.code_block_width = MAX_WIDTH - 1; // 1 space of padding at the end
                    return;
                } else if next_word_str_len > self.code_block_width {
                    current_line.clear();
                    current_line.push_str(indent);
                    continue;
                }
            }
        }
        self.code_block_single_quote_section = false;
        self.code_block_double_quote_section = false;
    }

    /// Pad the end of code block lines to the width to maintain a constant appearance
    fn pad_code_block_line(&mut self, formatted_line: &mut String, unformatted_line_len: usize) {
        if unformatted_line_len < self.code_block_width {
            *formatted_line += &" ".repeat(self.code_block_width - unformatted_line_len);
        }
        formatted_line.push_str(END_CODE_BLOCK_SECTION_ESC);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_formatter_new() {
        let formatter = PromptFormatter::new();
        assert!(formatter.formatted_prompt.is_empty());
        assert!(!formatter.bold_section);
        assert!(!formatter.code_block_section);
        assert!(!formatter.first_code_block_line);
        assert_eq!(formatter.language, "");
        assert!(!formatter.single_line_comment_section);
        assert!(!formatter.multi_line_comment_section);
        assert!(!formatter.code_block_double_quote_section);
        assert!(!formatter.code_block_single_quote_section);
        assert_eq!(formatter.width, DEFAULT_WIDTH);
        assert_eq!(formatter.code_block_width, DEFAULT_WIDTH);
    }

    #[test]
    fn test_prompt_formatter_default() {
        let formatter = PromptFormatter::default();
        assert!(formatter.formatted_prompt.is_empty());
        assert_eq!(formatter.width, DEFAULT_WIDTH);
    }

    #[test]
    fn test_format_prompt_simple() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("Hello world", 80);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "Hello world");
    }

    #[test]
    fn test_format_prompt_empty() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("", 80);
        assert_eq!(result.len(), 1);
        assert!(result[0].is_empty());
    }

    #[test]
    fn test_format_prompt_multiple_lines() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("Line 1\nLine 2\nLine 3", 80);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Line 1");
        assert_eq!(result[1], "Line 2");
        assert_eq!(result[2], "Line 3");
    }

    #[test]
    fn test_format_prompt_with_bold() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("This is **bold** text", 80);
        assert_eq!(result.len(), 1);
        assert!(result[0].contains(BOLD_TEXT_ESC));
        assert!(result[0].contains(NON_BOLD_TEXT_ESC));
    }

    #[test]
    fn test_format_prompt_with_code_block() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("```rust\nfn main() {}\n```", 80);
        assert!(result.len() >= 2);
        // Should contain code block formatting
        let joined = result.join("");
        assert!(joined.contains(START_CODE_BLOCK_SECTION_ESC));
        assert!(joined.contains(END_CODE_BLOCK_SECTION_ESC));
    }

    #[test]
    fn test_format_prompt_line_wrapping() {
        let mut formatter = PromptFormatter::new();
        let long_text = "This is a very long line that should be wrapped at the specified width because it exceeds the maximum characters per line limit";
        let result = formatter.format_prompt(long_text, 40);
        assert!(result.len() > 1);
    }

    #[test]
    fn test_format_prompt_preserves_indentation() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("    Indented line", 80);
        assert_eq!(result.len(), 1);
        assert!(result[0].starts_with("    "));
    }

    #[test]
    fn test_format_prompt_empty_lines() {
        let mut formatter = PromptFormatter::new();
        let result = formatter.format_prompt("Line 1\n\nLine 3", 80);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Line 1");
        assert!(result[1].is_empty());
        assert_eq!(result[2], "Line 3");
    }

    #[test]
    fn test_format_prompt_reuse_formatter() {
        let mut formatter = PromptFormatter::new();

        let result1 = formatter.format_prompt("First text", 80);
        assert_eq!(result1.len(), 1);

        let result2 = formatter.format_prompt("Second text", 80);
        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0], "Second text");
    }

    #[test]
    fn test_debug_implementation() {
        let formatter = PromptFormatter::new();
        let debug_str = format!("{:?}", formatter);
        assert!(debug_str.contains("PromptFormatter"));
        assert!(debug_str.contains("formatted_prompt"));
        assert!(debug_str.contains("bold_section"));
    }

    #[test]
    fn test_determine_max_width() {
        let mut formatter = PromptFormatter::new();
        formatter.determine_max_width("Short");
        assert_eq!(formatter.code_block_width, DEFAULT_WIDTH);

        // Test with very long line that should increase width
        let long_line = "a".repeat(100);
        formatter.determine_max_width(&long_line);
        assert!(formatter.code_block_width >= DEFAULT_WIDTH);
    }
}
