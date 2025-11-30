use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::Client;
use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, Message, SystemContentBlock};
use serde_json::Value;

pub struct LlmClient {
    client: Client,
    model_id: String,
}

impl LlmClient {
    pub async fn new() -> Result<Self> {
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        let client = Client::new(&config);
        
        Ok(Self {
            client,
            model_id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(), // Default to Claude 3 Sonnet
        })
    }

    pub async fn chat(&self, messages: Vec<Message>, system_prompt: Option<String>) -> Result<String> {
        let mut request = self.client
            .converse()
            .model_id(&self.model_id)
            .set_messages(Some(messages));

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
}
