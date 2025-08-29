use std::os::fd::OwnedFd;
use crate::create_prompt_record;
use crate::{CoreConfig, Prompt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

pub struct OpenAIChatCompletionRequest {
    pub role: String,
    pub messages: Vec<OpenAIMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
    pub service_tier: String,
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
    pub refusal: Option<String>,
    pub annotations: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: OpenAIPromptTokensDetails,
    pub completion_tokens_details: OpenAICompletionTokensDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIPromptTokensDetails {
    pub cached_tokens: u32,
    pub audio_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompletionTokensDetails {
    pub reasoning_tokens: u32,
    pub audio_tokens: u32,
    pub accepted_prediction_tokens: u32,
    pub rejected_prediction_tokens: u32,
}

pub async fn call_openai(
    prompt: &Prompt,
    model: &str,
    pool: &MySqlPool,
    //temperature: f64,
    //max_tokens: u32,
    //top_p: f64,
    //frequency_penalty: f64,
    //presence_penalty: f64,
    //stop: Option<Vec<String>>,
) -> Result<Prompt, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();
    let client = Client::new();

    if config.openai_key.is_none() {
        return Err("ANTHROPIC_KEY is not set".into());
    }
}

pub async fn get_openai_models() -> Result<Vec<OpenAIModel>, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();

    let client = Client::new();
    let response = client
        .get(&config.openai_url)
        .header("Authorization", format!("Bearer {}", &config.openai_key.as_ref().unwrap()))
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if response.status().is_success() {
        let models: Vec<OpenAIModel> = response.json().await?;
        Ok(models)
    } else {
        let error_text = response.text().await?;
        Err(format!("API request failed: {}", error_text).into())
    }
}