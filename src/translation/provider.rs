use anyhow::{Context, Result};
use reqwest::blocking::Client;

/// A translation provider that can translate text via an online API.
pub(crate) trait TranslationProvider: Send + Sync {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String>;
}

/// DeepL translation provider.
pub(crate) struct DeepLProvider {
    client: Client,
    endpoint: String,
    api_key: String,
}

impl DeepLProvider {
    pub(crate) fn new(endpoint: String, api_key: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            endpoint,
            api_key,
        }
    }
}

impl TranslationProvider for DeepLProvider {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        let resp = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("DeepL-Auth-Key {}", self.api_key))
            .form(&[
                ("text", text),
                ("source_lang", source_lang),
                ("target_lang", target_lang),
            ])
            .send()
            .context("DeepL translation request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("DeepL API error {status}: {body}");
        }

        let body: serde_json::Value = resp
            .json()
            .context("Failed to parse DeepL response")?;

        body.get("translations")
            .and_then(|t| t.get(0))
            .and_then(|t| t.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .context("DeepL response missing translation text")
    }
}

/// LLM (OpenAI-compatible) translation provider.
pub(crate) struct LLMProvider {
    client: Client,
    endpoint: String,
    api_key: String,
    model: String,
}

impl LLMProvider {
    pub(crate) fn new(endpoint: String, api_key: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            endpoint,
            api_key,
            model,
        }
    }
}

impl TranslationProvider for LLMProvider {
    fn translate(&self, text: &str, source_lang: &str, target_lang: &str) -> Result<String> {
        let prompt = format!(
            "Translate the following text from {} to {}. \
             Preserve all markdown formatting (bold, italic, links, code, etc.). \
             Output only the translation, nothing else.\n\n{}",
            source_lang, target_lang, text
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a translation assistant. Translate text accurately while preserving markdown formatting."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .context("LLM translation request failed")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("LLM API error {status}: {body}");
        }

        let resp_body: serde_json::Value = resp
            .json()
            .context("Failed to parse LLM response")?;

        resp_body
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.trim().to_string())
            .map(|s| s.trim_start_matches("\\n").to_string())
            .context("LLM response missing translation text")
    }
}

/// Build a provider from config values.
pub(crate) fn build_provider(
    provider_type: &str,
    api_endpoint: &str,
    api_key: &str,
) -> Box<dyn TranslationProvider> {
    match provider_type {
        "llm" => {
            let model = if api_endpoint.contains("openai") {
                "gpt-4o-mini".to_string()
            } else {
                "Qwen3-32B".to_string()
            };
            Box::new(LLMProvider::new(
                api_endpoint.to_string(),
                api_key.to_string(),
                model,
            ))
        }
        _ => Box::new(DeepLProvider::new(
            api_endpoint.to_string(),
            api_key.to_string(),
        )),
    }
}
