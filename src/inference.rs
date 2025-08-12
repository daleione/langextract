//! Simple library for performing language model inference.

use crate::data::FormatType;
use async_trait::async_trait;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

const OLLAMA_DEFAULT_MODEL_URL: &str = "http://localhost:11434";

/// Scored output from language model inference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredOutput {
    pub score: Option<f64>,
    pub output: Option<String>,
}

impl ScoredOutput {
    pub fn new(score: Option<f64>, output: Option<String>) -> Self {
        Self { score, output }
    }
}

impl std::fmt::Display for ScoredOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.score, &self.output) {
            (Some(score), Some(output)) => {
                write!(f, "Score: {:.2}\nOutput:\n  {}", score, output.replace('\n', "\n  "))
            }
            (Some(score), None) => write!(f, "Score: {:.2}\nOutput: None", score),
            (None, Some(output)) => write!(f, "Score: None\nOutput:\n  {}", output.replace('\n', "\n  ")),
            (None, None) => write!(f, "Score: None\nOutput: None"),
        }
    }
}

/// Exception raised when no scored outputs are available from the language model.
#[derive(Error, Debug)]
#[error("Inference output error: {message}")]
pub struct InferenceOutputError {
    pub message: String,
}

impl InferenceOutputError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Inference type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InferenceType {
    Iterative,
    Multiprocess,
}

/// An abstract inference trait for managing LLM inference.
#[async_trait]
pub trait BaseLanguageModel: Send + Sync {
    /// Implements language model inference.
    ///
    /// # Arguments
    /// * `batch_prompts` - Batch of inputs for inference. Single element vec can be used for a single input.
    /// * `kwargs` - Additional arguments for inference, like temperature and max_decode_steps.
    ///
    /// # Returns
    /// Batch of sequences of probable output text outputs, sorted by descending score.
    async fn infer(
        &self,
        batch_prompts: &[String],
        _kwargs: Option<HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<Vec<Vec<ScoredOutput>>, InferenceOutputError>;
}


/// Language model inference using OpenAI's API with structured output.
#[derive(Debug, Clone)]
pub struct OpenAILanguageModel {
    model_id: String,
    api_key: String,
    base_url: Option<String>,
    organization: Option<String>,
    format_type: FormatType,
    temperature: f64,
    max_workers: usize,
    extra_kwargs: HashMap<String, serde_json::Value>,
}

impl OpenAILanguageModel {
    pub fn new(
        model_id: Option<String>,
        api_key: String,
        base_url: Option<String>,
        organization: Option<String>,
        format_type: Option<FormatType>,
        temperature: Option<f64>,
        max_workers: Option<usize>,
        extra_kwargs: Option<HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<Self, InferenceOutputError> {
        if api_key.is_empty() {
            return Err(InferenceOutputError::new("API key not provided."));
        }

        Ok(Self {
            model_id: model_id.unwrap_or_else(|| "gpt-4o-mini".to_string()),
            api_key,
            base_url,
            organization,
            format_type: format_type.unwrap_or(FormatType::Json),
            temperature: temperature.unwrap_or(0.0),
            max_workers: max_workers.unwrap_or(10),
            extra_kwargs: extra_kwargs.unwrap_or_default(),
        })
    }

    async fn process_single_prompt(
        &self,
        prompt: &str,
        config: &HashMap<String, serde_json::Value>,
    ) -> std::result::Result<ScoredOutput, InferenceOutputError> {
        let client = reqwest::Client::new();
        let url = self.base_url.as_deref().unwrap_or("https://api.openai.com").to_string() + "/v1/chat/completions";

        let system_message = match self.format_type {
            FormatType::Json => "You are a helpful assistant that responds in JSON format.",
            FormatType::Yaml => "You are a helpful assistant that responds in YAML format.",
        };

        let mut request_body = serde_json::json!({
            "model": self.model_id,
            "messages": [
                {"role": "system", "content": system_message},
                {"role": "user", "content": prompt}
            ],
            "temperature": config.get("temperature").and_then(|v| v.as_f64()).unwrap_or(self.temperature),
            "n": 1
        });

        if let Some(max_tokens) = config.get("max_output_tokens").and_then(|v| v.as_i64()) {
            request_body["max_tokens"] = serde_json::Value::Number(serde_json::Number::from(max_tokens));
        }
        if let Some(top_p) = config.get("top_p").and_then(|v| v.as_f64()) {
            request_body["top_p"] = serde_json::Value::Number(serde_json::Number::from_f64(top_p).unwrap());
        }

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| InferenceOutputError::new(e.to_string()))?;

        if !response.status().is_success() {
            return Err(InferenceOutputError::new(format!(
                "OpenAI API error: {}",
                response.status()
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| InferenceOutputError::new(e.to_string()))?;
        let output_text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string());

        Ok(ScoredOutput::new(Some(1.0), output_text))
    }

    pub fn parse_output(&self, output: &str) -> std::result::Result<serde_json::Value, InferenceOutputError> {
        match self.format_type {
            FormatType::Json => serde_json::from_str(output)
                .map_err(|e| InferenceOutputError::new(format!("Failed to parse output as JSON: {}", e))),
            FormatType::Yaml => serde_yaml::from_str(output)
                .map_err(|e| InferenceOutputError::new(format!("Failed to parse output as YAML: {}", e))),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for OpenAILanguageModel {
    async fn infer(
        &self,
        batch_prompts: &[String],
        kwargs: Option<HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<Vec<Vec<ScoredOutput>>, InferenceOutputError> {
        let config = kwargs.unwrap_or_default();

        if batch_prompts.len() > 1 && self.max_workers > 1 {
            // Parallel processing
            let tasks: Vec<_> = batch_prompts
                .iter()
                .map(|prompt| self.process_single_prompt(prompt, &config))
                .collect();

            let results = try_join_all(tasks)
                .await
                .map_err(|e| InferenceOutputError::new(e.to_string()))?;
            Ok(results.into_iter().map(|r| vec![r]).collect())
        } else {
            // Sequential processing
            let mut results = Vec::new();
            for prompt in batch_prompts {
                let r = self
                    .process_single_prompt(prompt, &config)
                    .await
                    .map_err(|e| InferenceOutputError::new(e.to_string()))?;
                results.push(vec![r]);
            }
            Ok(results)
        }
    }
}

/// Language model inference using DeepSeek's API with structured output.
#[derive(Debug, Clone)]
pub struct DeepSeekLanguageModel {
    model_id: String,
    api_key: String,
    base_url: String,
    format_type: FormatType,
    temperature: f64,
    max_workers: usize,
    extra_kwargs: HashMap<String, serde_json::Value>,
}

impl DeepSeekLanguageModel {
    pub fn new(
        model_id: Option<String>,
        api_key: String,
        base_url: Option<String>,
        format_type: Option<FormatType>,
        temperature: Option<f64>,
        max_workers: Option<usize>,
        extra_kwargs: Option<HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<Self, InferenceOutputError> {
        if api_key.is_empty() {
            return Err(InferenceOutputError::new("API key not provided."));
        }

        Ok(Self {
            model_id: model_id.unwrap_or_else(|| "deepseek-chat".to_string()),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.deepseek.com".to_string()),
            format_type: format_type.unwrap_or(FormatType::Json),
            temperature: temperature.unwrap_or(0.0),
            max_workers: max_workers.unwrap_or(10),
            extra_kwargs: extra_kwargs.unwrap_or_default(),
        })
    }

    async fn process_single_prompt(
        &self,
        prompt: &str,
        config: &HashMap<String, serde_json::Value>,
    ) -> std::result::Result<ScoredOutput, InferenceOutputError> {
        let client = reqwest::Client::new();
        let url = format!("{}/v1/chat/completions", self.base_url);

        let system_message = match self.format_type {
            FormatType::Json => "You are a helpful assistant that responds in JSON format.",
            FormatType::Yaml => "You are a helpful assistant that responds in YAML format.",
        };

        let mut request_body = serde_json::json!({
            "model": self.model_id,
            "messages": [
                {"role": "system", "content": system_message},
                {"role": "user", "content": prompt}
            ],
            "temperature": config.get("temperature").and_then(|v| v.as_f64()).unwrap_or(self.temperature),
            "stream": false
        });

        if let Some(max_tokens) = config.get("max_output_tokens").and_then(|v| v.as_i64()) {
            request_body["max_tokens"] = serde_json::Value::Number(serde_json::Number::from(max_tokens));
        }
        if let Some(top_p) = config.get("top_p").and_then(|v| v.as_f64()) {
            request_body["top_p"] = serde_json::Value::Number(serde_json::Number::from_f64(top_p).unwrap());
        }

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| InferenceOutputError::new(e.to_string()))?;

        if !response.status().is_success() {
            return Err(InferenceOutputError::new(format!(
                "DeepSeek API error: {}",
                response.status()
            )));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| InferenceOutputError::new(e.to_string()))?;
        let output_text = response_json["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string());

        Ok(ScoredOutput::new(Some(1.0), output_text))
    }

    pub fn parse_output(&self, output: &str) -> std::result::Result<serde_json::Value, InferenceOutputError> {
        match self.format_type {
            FormatType::Json => serde_json::from_str(output)
                .map_err(|e| InferenceOutputError::new(format!("Failed to parse output as JSON: {}", e))),
            FormatType::Yaml => serde_yaml::from_str(output)
                .map_err(|e| InferenceOutputError::new(format!("Failed to parse output as YAML: {}", e))),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for DeepSeekLanguageModel {
    async fn infer(
        &self,
        batch_prompts: &[String],
        kwargs: Option<HashMap<String, serde_json::Value>>,
    ) -> std::result::Result<Vec<Vec<ScoredOutput>>, InferenceOutputError> {
        let config = kwargs.unwrap_or_default();

        if batch_prompts.len() > 1 && self.max_workers > 1 {
            // Parallel processing
            let tasks: Vec<_> = batch_prompts
                .iter()
                .map(|prompt| self.process_single_prompt(prompt, &config))
                .collect();

            let results = try_join_all(tasks)
                .await
                .map_err(|e| InferenceOutputError::new(e.to_string()))?;
            Ok(results.into_iter().map(|r| vec![r]).collect())
        } else {
            // Sequential processing
            let mut results = Vec::new();
            for prompt in batch_prompts {
                let r = self
                    .process_single_prompt(prompt, &config)
                    .await
                    .map_err(|e| InferenceOutputError::new(e.to_string()))?;
                results.push(vec![r]);
            }
            Ok(results)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_scored_output_creation() {
        let output = ScoredOutput::new(Some(0.85), Some("test output".to_string()));
        assert_eq!(output.score, Some(0.85));
        assert_eq!(output.output, Some("test output".to_string()));
    }

    #[test]
    fn test_scored_output_display() {
        let output = ScoredOutput::new(Some(0.85), Some("test output".to_string()));
        let display_str = format!("{}", output);
        assert!(display_str.contains("Score: 0.85"));
        assert!(display_str.contains("test output"));
    }

    #[test]
    fn test_openai_model_creation() {
        let model = OpenAILanguageModel::new(None, "test-api-key".to_string(), None, None, None, None, None, None);
        assert!(model.is_ok());
        let model = model.unwrap();
        assert_eq!(model.model_id, "gpt-4o-mini");
        assert_eq!(model.api_key, "test-api-key");
        assert_eq!(model.temperature, 0.0);
        assert_eq!(model.max_workers, 10);
    }

    #[test]
    fn test_openai_model_empty_api_key() {
        let model = OpenAILanguageModel::new(None, "".to_string(), None, None, None, None, None, None);
        assert!(model.is_err());
        assert!(model.unwrap_err().to_string().contains("API key not provided"));
    }

    #[test]
    fn test_deepseek_model_creation() {
        let model = DeepSeekLanguageModel::new(None, "test-api-key".to_string(), None, None, None, None, None);
        assert!(model.is_ok());
        let model = model.unwrap();
        assert_eq!(model.model_id, "deepseek-chat");
        assert_eq!(model.api_key, "test-api-key");
        assert_eq!(model.base_url, "https://api.deepseek.com");
        assert_eq!(model.temperature, 0.0);
        assert_eq!(model.max_workers, 10);
    }

    #[test]
    fn test_deepseek_model_empty_api_key() {
        let model = DeepSeekLanguageModel::new(None, "".to_string(), None, None, None, None, None);
        assert!(model.is_err());
        assert!(model.unwrap_err().to_string().contains("API key not provided"));
    }

    #[test]
    fn test_openai_parse_output_json() {
        let model = OpenAILanguageModel::new(
            None,
            "test-key".to_string(),
            None,
            None,
            Some(FormatType::Json),
            None,
            None,
            None,
        )
        .unwrap();

        let json_output = r#"{"key": "value", "number": 42}"#;
        let result = model.parse_output(json_output);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["number"], 42);
    }

    #[test]
    fn test_openai_parse_output_yaml() {
        let model = OpenAILanguageModel::new(
            None,
            "test-key".to_string(),
            None,
            None,
            Some(FormatType::Yaml),
            None,
            None,
            None,
        )
        .unwrap();

        let yaml_output = "key: value\nnumber: 42";
        let result = model.parse_output(yaml_output);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["number"], 42);
    }

    #[test]
    fn test_deepseek_parse_output_json() {
        let model = DeepSeekLanguageModel::new(
            None,
            "test-key".to_string(),
            None,
            Some(FormatType::Json),
            None,
            None,
            None,
        )
        .unwrap();

        let json_output = r#"{"key": "value", "number": 42}"#;
        let result = model.parse_output(json_output);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed["key"], "value");
        assert_eq!(parsed["number"], 42);
    }

    #[test]
    fn test_inference_output_error() {
        let error = InferenceOutputError::new("Test error message");
        assert_eq!(error.message, "Test error message");
        assert!(error.to_string().contains("Test error message"));
    }

    #[test]
    fn test_inference_type_serialization() {
        let iterative = InferenceType::Iterative;
        let multiprocess = InferenceType::Multiprocess;

        let iterative_json = serde_json::to_string(&iterative).unwrap();
        let multiprocess_json = serde_json::to_string(&multiprocess).unwrap();

        assert_eq!(iterative_json, "\"iterative\"");
        assert_eq!(multiprocess_json, "\"multiprocess\"");
    }
}
