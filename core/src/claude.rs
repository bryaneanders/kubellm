use serde::{Deserialize, Serialize};
use crate::{Config, CreatePromptResponse};
use reqwest::Client;
use sqlx::MySqlPool;
use crate::create_prompt_record;

#[derive(Debug, Deserialize)]
pub struct ClaudeResponse {
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub role: String,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub struct ContentBlock {
    pub text: String,
    #[serde(rename = "type")]
    pub content_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

// Request structures
#[derive(Debug, Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub async fn call_claude(prompt: &str, pool: &MySqlPool,) -> Result<CreatePromptResponse, Box<dyn std::error::Error>> {
    let config = Config::get();
    let client = Client::new();

    let request = ClaudeRequest {
        model: "claude-sonnet-4-20250514".to_string(), // make this dynamic later
        max_tokens: 1024,
        messages: vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(&format!("{}/messages", &config.anthropic_url))
        .header("x-api-key", &config.anthropic_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    // Check if request was successful
    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("API request failed: {}", error_text).into());
    }

    // Parse the response
    let claude_response: ClaudeResponse = response.json().await?;

    // Extract the text from the first content block
    let response_text = claude_response
        .content
        .first()
        .map(|block| block.text.clone())
        .unwrap_or_else(|| "No response content".to_string());

    Ok(create_prompt_record(pool, prompt.to_string(), Some(&response_text)).await?)
}