use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::Client;
use aws_sdk_bedrockruntime::types::{ContentBlock, Message as BedrockMessage, SystemContentBlock, ConversationRole};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

pub enum LlmProvider {
    Bedrock,
    Ollama,
}

pub struct LlmClient {
    client: Option<Client>, // Optional because Ollama doesn't need it
    model_id: String,
    provider: LlmProvider,
    ollama_url: String,
}

impl LlmClient {
    pub async fn new() -> Result<Self> {
        let provider_str = std::env::var("LLM_PROVIDER").unwrap_or_else(|_| "bedrock".to_string());
        
        let (provider, client) = match provider_str.to_lowercase().as_str() {
            "ollama" => (LlmProvider::Ollama, None),
            _ => {
                let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
                (LlmProvider::Bedrock, Some(Client::new(&config)))
            }
        };

        let model_id = std::env::var("MODEL_ID").unwrap_or_else(|_| {
            match provider {
                LlmProvider::Bedrock => "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
                LlmProvider::Ollama => "llama3.1".to_string(),
            }
        });
        
        let ollama_url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434/api/chat".to_string());

        Ok(Self {
            client,
            model_id,
            provider,
            ollama_url,
        })
    }

    pub async fn chat(&self, messages: Vec<Message>, system_prompt: Option<String>) -> Result<String> {
        match self.provider {
            LlmProvider::Bedrock => self.chat_bedrock(messages, system_prompt).await,
            LlmProvider::Ollama => self.chat_ollama(messages, system_prompt).await,
        }
    }

    async fn chat_bedrock(&self, messages: Vec<Message>, system_prompt: Option<String>) -> Result<String> {
        let client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("Bedrock client not initialized"))?;
        
        // Convert generic messages to Bedrock messages
        let bedrock_messages: Vec<BedrockMessage> = messages.into_iter().map(|m| {
            let role = match m.role {
                Role::User => ConversationRole::User,
                Role::Assistant => ConversationRole::Assistant,
            };
            BedrockMessage::builder()
                .role(role)
                .content(ContentBlock::Text(m.content))
                .build()
                .unwrap() // Should be safe
        }).collect();

        let mut request = client
            .converse()
            .model_id(&self.model_id)
            .set_messages(Some(bedrock_messages));

        if let Some(prompt) = system_prompt {
             let system_block = SystemContentBlock::Text(prompt);
             request = request.system(system_block);
        }

        let output = request.send().await.map_err(|e| anyhow::anyhow!("Bedrock error: {}", e))?;

        if let Some(output_message) = output.output {
            match output_message {
                aws_sdk_bedrockruntime::types::ConverseOutput::Message(message) => {
                     if let Some(content) = message.content.first() {
                         match content {
                             ContentBlock::Text(text) => return Ok(text.clone()),
                             _ => return Ok("Received non-text response".to_string()),
                         }
                     }
                }
                _ => {}
            }
        }

        Ok("No response generated".to_string())
    }

    async fn chat_ollama(&self, messages: Vec<Message>, system_prompt: Option<String>) -> Result<String> {
        let client = reqwest::Client::new();
        
        // Ollama format:
        // { "model": "llama3", "messages": [ { "role": "user", "content": "..." } ], "stream": false }
        // System prompt is usually just another message with role "system" at the start
        
        let mut ollama_messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            ollama_messages.push(serde_json::json!({
                "role": "system",
                "content": prompt
            }));
        }
        
        for msg in messages {
            let role = match msg.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };
            ollama_messages.push(serde_json::json!({
                "role": role,
                "content": msg.content
            }));
        }

        let payload = serde_json::json!({
            "model": self.model_id,
            "messages": ollama_messages,
            "stream": false
        });

        let resp = client.post(&self.ollama_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Ollama request error: {}", e))?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Ollama API error: {}", resp.status()));
        }

        let resp_json: serde_json::Value = resp.json().await
            .map_err(|e| anyhow::anyhow!("Failed to parse Ollama response: {}", e))?;

        // Extract content from response
        // Response format: { "message": { "role": "assistant", "content": "..." }, ... }
        
        if let Some(content) = resp_json.get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str()) {
            Ok(content.to_string())
        } else {
            Err(anyhow::anyhow!("Invalid response format from Ollama"))
        }
    }
}
