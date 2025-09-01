use lazy_static::lazy_static;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Java,
    Bash,
}

impl Language {
    pub fn from_string(lang: &str) -> Option<Language> {
        match lang.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "java" => Some(Language::Java),
            "bash" | "sh" | "shell" => Some(Language::Bash),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Java => "java",
            Language::Bash => "bash",
        }
    }
}

lazy_static! {
    static ref LANGUAGE_KEYWORDS: HashMap<Language, HashSet<&'static str>> = {
        let mut map = HashMap::new();

        // Rust keywords
        let rust_keywords: HashSet<&'static str> = [
            // Strict keywords
            "as", "break", "const", "continue", "crate", "else", "enum", "extern",
            "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
            "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
            "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
            "while", "async", "await", "dyn",
            // Reserved keywords
            "abstract", "become", "box", "do", "final", "macro", "override",
            "priv", "typeof", "unsized", "virtual", "yield", "try",
        ].iter().cloned().collect();

        // Java keywords
        let java_keywords: HashSet<&'static str> = [
            "abstract", "assert", "boolean", "break", "byte", "case", "catch",
            "char", "class", "const", "continue", "default", "do", "double",
            "else", "enum", "extends", "final", "finally", "float", "for",
            "goto", "if", "implements", "import", "instanceof", "int", "interface",
            "long", "native", "new", "package", "private", "protected", "public",
            "return", "short", "static", "strictfp", "super", "switch", "synchronized",
            "this", "throw", "throws", "transient", "try", "void", "volatile", "while",
            // Reserved literals
            "true", "false", "null",
            // Module system keywords (Java 9+)
            "module", "requires", "exports", "opens", "uses", "provides", "transitive",
            // Contextual keywords
            "var", "yield", "record", "sealed", "permits", "non-sealed",
        ].iter().cloned().collect();

        // Bash keywords and built-ins
        let bash_keywords: HashSet<&'static str> = [
            // Shell keywords
            "if", "then", "else", "elif", "fi", "case", "esac", "for", "select",
            "while", "until", "do", "done", "function", "in", "time", "coproc",
            // Conditional constructs
            /*"[[", "]]", "[", "]",*/
            // Built-in commands (commonly considered keywords)
            "alias", "bg", "bind", "break", "builtin", "caller", "cd", "command",
            "compgen", "complete", "compopt", "continue", "declare", "dirs",
            "disown", "echo", "enable", "eval", "exec", "exit", "export", "fc",
            "fg", "getopts", "hash", "help", "history", "jobs", "kill", "let",
            "local", "logout", "mapfile", "popd", "printf", "pushd", "pwd",
            "read", "readonly", "return", "set", "shift", "shopt", "source",
            "suspend", "test", "trap", "type", "typeset", "ulimit", "umask",
            "unalias", "unset", "wait",
            // Reserved words
            /*"!", "{", "}", "&&", "||",*/
        ].iter().cloned().collect();

        map.insert(Language::Rust, rust_keywords);
        map.insert(Language::Java, java_keywords);
        map.insert(Language::Bash, bash_keywords);

        map
    };
}

pub struct KeywordChecker;

impl KeywordChecker {
    /// Check if a word is a keyword in the specified language
    pub fn is_keyword(word: &str, language: &str) -> Result<bool, String> {
        let lang = Language::from_string(language)
            .ok_or_else(|| format!("Unsupported language: {}", language))?;

        Ok(LANGUAGE_KEYWORDS
            .get(&lang)
            .map(|keywords| keywords.contains(word))
            .unwrap_or(false))
    }

    /// Check if a word is a keyword using Language enum directly
    pub fn is_keyword_enum(word: &str, language: Language) -> bool {
        LANGUAGE_KEYWORDS
            .get(&language)
            .map(|keywords| keywords.contains(word))
            .unwrap_or(false)
    }

    /// Get all keywords for a specific language
    pub fn get_keywords(language: &str) -> Result<Vec<&'static str>, String> {
        let lang = Language::from_string(language)
            .ok_or_else(|| format!("Unsupported language: {}", language))?;

        Ok(LANGUAGE_KEYWORDS
            .get(&lang)
            .map(|keywords| {
                let mut kw_vec: Vec<&str> = keywords.iter().copied().collect();
                kw_vec.sort();
                kw_vec
            })
            .unwrap_or_default())
    }

    /// Get supported languages
    pub fn supported_languages() -> Vec<&'static str> {
        vec!["rust", "java", "bash"]
    }

    /// Check multiple words at once
    pub fn check_multiple(words: &[&str], language: &str) -> Result<HashMap<String, bool>, String> {
        let lang = Language::from_string(language)
            .ok_or_else(|| format!("Unsupported language: {}", language))?;

        let keywords = LANGUAGE_KEYWORDS.get(&lang).unwrap();

        Ok(words
            .iter()
            .map(|word| (word.to_string(), keywords.contains(word)))
            .collect())
    }
}