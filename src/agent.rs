use anyhow::Result;
use aws_sdk_bedrockruntime::types::{ContentBlock, ConversationRole, Message};
use crate::llm::LlmClient;

pub struct Agent {
    llm: LlmClient,
    history: Vec<Message>,
    system_prompt: String,
}

impl Agent {
    pub async fn new(system_prompt: &str) -> Result<Self> {
        Ok(Self {
            llm: LlmClient::new().await?,
            history: Vec::new(),
            system_prompt: system_prompt.to_string(),
        })
    }

    pub async fn chat(&mut self, user_input: &str) -> Result<String> {
        // Add user message to history
        let user_msg = Message::builder()
            .role(ConversationRole::User)
            .content(ContentBlock::Text(user_input.to_string()))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build user message: {}", e))?;
        
        self.history.push(user_msg);

        // Get response from LLM
        let response_text = self.llm.chat(self.history.clone(), Some(self.system_prompt.clone())).await?;

        // Add assistant response to history
        let assistant_msg = Message::builder()
            .role(ConversationRole::Assistant)
            .content(ContentBlock::Text(response_text.clone()))
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build assistant message: {}", e))?;
        
        self.history.push(assistant_msg);

        Ok(response_text)
    }
}
