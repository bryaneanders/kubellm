use crate::create_prompt_record;
use crate::{CoreConfig, Prompt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIModelsResponse {
    pub object: String,
    pub data: Vec<OpenAIModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIModel {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

impl OpenAIModelsResponse {
    /// Filter models that start with any valid TextModelType prefix
    pub fn filter_by_valid_prefixes(&self) -> Vec<OpenAIModel> {
        let valid_prefixes = TextModelType::all_prefixes();

        let filtered_models: Vec<&OpenAIModel> = self
            .data
            .iter()
            .filter(|model| {
                let model_id = model.id.to_lowercase();
                valid_prefixes
                    .iter()
                    .any(|prefix| model_id.starts_with(prefix))
            })
            .collect();

        filtered_models.into_iter().cloned().collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

impl OpenAIChatRequest {
    pub fn new(model: String, messages: Vec<OpenAIMessage>) -> Self {
        Self {
            model,
            messages,
            temperature: None,
            max_tokens: None,
            max_completion_tokens: None,
            //additional: Map::new(),
        }
    }

    // newer models use max_completion_tokens, not max_tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        if self.is_newer_model() {
            self.max_completion_tokens = Some(max_tokens);
            self.max_tokens = None;
        } else {
            self.max_tokens = Some(max_tokens);
            self.max_completion_tokens = None;
        }
        self
    }

    pub fn with_temperature(mut self, temperature: Option<f32>) -> Self {
        if self.is_newer_model() {
            self.temperature = Some(1.0);
        } else {
            self.temperature = temperature;
        }
        self
    }

    fn is_newer_model(&self) -> bool {
        let newer_model_prefixes = [
            "gpt-5", "o1",
            // Add more as they're released
        ];

        newer_model_prefixes
            .iter()
            .any(|prefix| self.model.starts_with(prefix))
    }
}

pub struct OpenAIChatRequestBuilder {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: Option<f32>,
    max_tokens_value: Option<u32>,
    //additional_params: Map<String, Value>,
}

impl OpenAIChatRequestBuilder {
    pub fn new(model: String) -> Self {
        Self {
            model,
            messages: Vec::new(),
            temperature: None,
            max_tokens_value: None,
            //additional_params: Map::new(),
        }
    }

    pub fn messages(mut self, messages: Vec<OpenAIMessage>) -> Self {
        self.messages = messages;
        self
    }

    pub fn add_message(mut self, role: &str, content: &str) -> Self {
        self.messages.push(OpenAIMessage {
            role: role.to_string(),
            content: content.to_string(),
        });
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens_value = Some(tokens);
        self
    }

    /*    pub fn additional_param<T: serde::Serialize>(mut self, key: &str, value: T) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.additional_params.insert(key.to_string(), json_value);
        }
        self
    }*/

    pub fn build(self) -> OpenAIChatRequest {
        let mut request = OpenAIChatRequest::new(self.model, self.messages);

        if let Some(temp) = self.temperature {
            request = request.with_temperature(Some(temp));
        }

        if let Some(max_tokens) = self.max_tokens_value {
            request = request.with_max_tokens(max_tokens);
        }

        //request.additional = self.additional_params;
        request
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChoice>,
    pub usage: OpenAIUsage,
    pub service_tier: String,
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
    /*pub refusal: Option<String>,
    pub annotations: Vec<serde_json::Value>,*/
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub prompt_tokens_details: OpenAIPromptTokensDetails,
    pub completion_tokens_details: OpenAICompletionTokensDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAIPromptTokensDetails {
    pub cached_tokens: u32,
    pub audio_tokens: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenAICompletionTokensDetails {
    pub reasoning_tokens: u32,
    pub audio_tokens: u32,
    pub accepted_prediction_tokens: u32,
    pub rejected_prediction_tokens: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TextModelType {
    GPT5,
    GPT4o,
    GPT4,
    GPT35,
    O1,
    // not including legacy models for now
    /*Davinci,
    Curie,
    Babbage,
    Ada,
    NotTextModel,*/
}

impl FromStr for TextModelType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gpt-5" => Ok(TextModelType::GPT5),
            "gpt-4o" => Ok(TextModelType::GPT4o),
            "gpt-4" => Ok(TextModelType::GPT4),
            "gpt-3.5" => Ok(TextModelType::GPT35),
            "o1" => Ok(TextModelType::O1),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}

impl TextModelType {
    pub fn all_prefixes() -> Vec<&'static str> {
        vec!["gpt-5", "gpt-4o", "gpt-4", "gpt-3.5", "o1"]
    }

    pub fn to_prefix(&self) -> &'static str {
        match self {
            TextModelType::GPT5 => "gpt-5",
            TextModelType::GPT4o => "gpt-4o",
            TextModelType::GPT4 => "gpt-4",
            TextModelType::GPT35 => "gpt-3.5",
            TextModelType::O1 => "o1",
        }
    }
}

pub async fn call_openai(
    prompt: &str,
    model: Option<&str>,
    pool: &MySqlPool,
) -> Result<Prompt, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();
    let client = Client::new();

    if config.openai_key.is_none() {
        return Err("ANTHROPIC_KEY is not set".into());
    }

    let mut model = model.unwrap_or(&config.default_openai_model);
    let models = get_openai_models().await?;
    if !models.iter().any(|m| m.id == model) {
        println!(
            "\r\x1b[2kInvalid model, {}, falling back to default model, {}",
            model, &config.default_openai_model
        );
        model = &config.default_openai_model;
    }

    let request = OpenAIChatRequestBuilder::new(model.to_string())
        //.add_message("system", "You are a helpful assistant")
        .add_message("user", prompt)
        .temperature(0.5)
        .max_tokens(500)
        //.additional_param("top_p", 0.9)
        //.additional_param("frequency_penalty", 0.1)
        .build();

    let response = client
        .post(format!("{}/chat/completions", &config.openai_url))
        .header(
            "authorization",
            format!("Bearer {}", &config.openai_key.as_ref().unwrap()),
        )
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let chat_response: OpenAIChatResponse = response.json().await?;
        if let Some(choice) = chat_response.choices.first() {
            let repose_text = choice.message.content.as_str();

            Ok(
                create_prompt_record(pool, prompt.to_string(), Some(repose_text), Some(model))
                    .await?,
            )
        } else {
            Err("No choices returned from OpenAI API".into())
        }
    } else {
        let error_text = response.text().await?;
        Err(format!("OpenAI API request failed: {}", error_text).into())
    }
}

pub async fn get_openai_models() -> Result<Vec<OpenAIModel>, Box<dyn std::error::Error>> {
    let config = CoreConfig::get();

    if config.openai_key.is_none() {
        return Err("OPENAI_KEY is not set".into());
    }

    let client = Client::new();
    let response = client
        .get(format!("{}/models", &config.openai_url))
        .header(
            "Authorization",
            format!("Bearer {}", &config.openai_key.clone().unwrap()),
        )
        .send()
        .await?;

    if response.status().is_success() {
        let models_response: OpenAIModelsResponse = response.json().await?;
        let mut models: Vec<OpenAIModel> = models_response.filter_by_valid_prefixes();
        models.sort_by(|a, b| a.id.cmp(&b.id));

        // see if model id starts with one of the TextModelType fromstrs

        Ok(models)
    } else {
        let error_text = response.text().await?;
        Err(format!("OpenAI API request failed: {}", error_text).into())
    }
}
