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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_string() {
        assert_eq!(Language::from_string("rust"), Some(Language::Rust));
        assert_eq!(Language::from_string("rs"), Some(Language::Rust));
        assert_eq!(Language::from_string("java"), Some(Language::Java));
        assert_eq!(Language::from_string("bash"), Some(Language::Bash));
        assert_eq!(Language::from_string("sh"), Some(Language::Bash));
        assert_eq!(Language::from_string("shell"), Some(Language::Bash));
        assert_eq!(Language::from_string("unknown"), None);
    }

    #[test]
    fn test_language_as_str() {
        assert_eq!(Language::Rust.as_str(), "rust");
        assert_eq!(Language::Java.as_str(), "java");
        assert_eq!(Language::Bash.as_str(), "bash");
    }

    #[test]
    fn test_language_debug() {
        let lang = Language::Rust;
        assert!(format!("{:?}", lang).contains("Rust"));
    }

    #[test]
    fn test_language_equality() {
        assert_eq!(Language::Rust, Language::Rust);
        assert_ne!(Language::Rust, Language::Java);
    }

    #[test]
    fn test_keyword_checker_is_keyword() {
        assert!(KeywordChecker::is_keyword("fn", "rust").unwrap());
        assert!(KeywordChecker::is_keyword("class", "java").unwrap());
        assert!(KeywordChecker::is_keyword("if", "bash").unwrap());
        assert!(!KeywordChecker::is_keyword("unknown", "rust").unwrap());
    }

    #[test]
    fn test_keyword_checker_is_keyword_enum() {
        assert!(KeywordChecker::is_keyword_enum("fn", Language::Rust));
        assert!(KeywordChecker::is_keyword_enum("class", Language::Java));
        assert!(KeywordChecker::is_keyword_enum("if", Language::Bash));
        assert!(!KeywordChecker::is_keyword_enum("unknown", Language::Rust));
    }

    #[test]
    fn test_keyword_checker_unsupported_language() {
        let result = KeywordChecker::is_keyword("test", "unsupported");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unsupported language"));
    }

    #[test]
    fn test_keyword_checker_get_keywords() {
        let rust_keywords = KeywordChecker::get_keywords("rust").unwrap();
        assert!(rust_keywords.contains(&"fn"));
        assert!(rust_keywords.contains(&"struct"));

        let java_keywords = KeywordChecker::get_keywords("java").unwrap();
        assert!(java_keywords.contains(&"class"));
        assert!(java_keywords.contains(&"public"));

        let bash_keywords = KeywordChecker::get_keywords("bash").unwrap();
        assert!(bash_keywords.contains(&"if"));
        assert!(bash_keywords.contains(&"then"));
    }

    #[test]
    fn test_keyword_checker_supported_languages() {
        let languages = KeywordChecker::supported_languages();
        assert!(languages.contains(&"rust"));
        assert!(languages.contains(&"java"));
        assert!(languages.contains(&"bash"));
        assert_eq!(languages.len(), 3);
    }

    #[test]
    fn test_keyword_checker_check_multiple() {
        let words = vec!["fn", "unknown", "struct"];
        let result = KeywordChecker::check_multiple(&words, "rust").unwrap();

        assert_eq!(result.get("fn"), Some(&true));
        assert_eq!(result.get("unknown"), Some(&false));
        assert_eq!(result.get("struct"), Some(&true));
    }

    #[test]
    fn test_keyword_checker_check_multiple_unsupported_language() {
        let words = vec!["test"];
        let result = KeywordChecker::check_multiple(&words, "unsupported");
        assert!(result.is_err());
    }

    #[test]
    fn test_rust_specific_keywords() {
        assert!(KeywordChecker::is_keyword("async", "rust").unwrap());
        assert!(KeywordChecker::is_keyword("await", "rust").unwrap());
        assert!(KeywordChecker::is_keyword("dyn", "rust").unwrap());
        assert!(KeywordChecker::is_keyword("impl", "rust").unwrap());
    }

    #[test]
    fn test_java_specific_keywords() {
        assert!(KeywordChecker::is_keyword("synchronized", "java").unwrap());
        assert!(KeywordChecker::is_keyword("instanceof", "java").unwrap());
        assert!(KeywordChecker::is_keyword("extends", "java").unwrap());
        assert!(KeywordChecker::is_keyword("implements", "java").unwrap());
    }

    #[test]
    fn test_bash_specific_keywords() {
        assert!(KeywordChecker::is_keyword("elif", "bash").unwrap());
        assert!(KeywordChecker::is_keyword("esac", "bash").unwrap());
        assert!(KeywordChecker::is_keyword("coproc", "bash").unwrap());
    }
}
