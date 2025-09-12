use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;
use strum::{Display, EnumIter, IntoEnumIterator};

// maps the json containing the prompt into this struct
#[derive(Deserialize)]
pub struct CreatePromptRequest {
    pub prompt: String,
    pub provider: String,
    pub model: Option<String>,
}

// Serialize: used to convert this struct into JSON for responses
// FromRow: maps the database row into this struct
#[derive(Serialize, FromRow)]
pub struct Prompt {
    pub id: i64,
    pub prompt: String,
    pub model: String,
    pub provider: String,
    pub response: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Deserialize)]
pub struct GetModelsQuery {
    pub provider: String,
}

#[derive(Display, EnumIter, Debug, PartialEq)]
pub enum Provider {
    #[strum(to_string = "Anthropic")]
    Anthropic,
    #[strum(to_string = "OpenAI")]
    OpenAI,
}

impl FromStr for Provider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "anthropic" => Ok(Provider::Anthropic),
            "openai" => Ok(Provider::OpenAI),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

impl Provider {
    pub fn all() -> Vec<Provider> {
        Provider::iter().collect()
    }

    pub fn all_names() -> Vec<String> {
        Provider::iter().map(|p| p.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(
            Provider::from_str("anthropic").unwrap(),
            Provider::Anthropic
        );
        assert_eq!(
            Provider::from_str("Anthropic").unwrap(),
            Provider::Anthropic
        );
        assert_eq!(
            Provider::from_str("ANTHROPIC").unwrap(),
            Provider::Anthropic
        );

        assert_eq!(Provider::from_str("openai").unwrap(), Provider::OpenAI);
        assert_eq!(Provider::from_str("OpenAI").unwrap(), Provider::OpenAI);
        assert_eq!(Provider::from_str("OPENAI").unwrap(), Provider::OpenAI);

        assert!(Provider::from_str("invalid").is_err());
        assert!(Provider::from_str("").is_err());
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::Anthropic.to_string(), "Anthropic");
        assert_eq!(Provider::OpenAI.to_string(), "OpenAI");
    }

    #[test]
    fn test_provider_all() {
        let providers = Provider::all();
        assert_eq!(providers.len(), 2);
        assert!(providers.contains(&Provider::Anthropic));
        assert!(providers.contains(&Provider::OpenAI));
    }

    #[test]
    fn test_provider_all_names() {
        let names = Provider::all_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Anthropic".to_string()));
        assert!(names.contains(&"OpenAI".to_string()));
    }

    #[test]
    fn test_provider_partial_eq() {
        assert_eq!(Provider::Anthropic, Provider::Anthropic);
        assert_eq!(Provider::OpenAI, Provider::OpenAI);
        assert_ne!(Provider::Anthropic, Provider::OpenAI);
    }
}
