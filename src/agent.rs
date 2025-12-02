use anyhow::Result;
use crate::llm::{LlmClient, Message, Role};

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
        let user_msg = Message {
            role: Role::User,
            content: user_input.to_string(),
        };
        
        self.history.push(user_msg);

        // Get response from LLM
        let response_text = self.llm.chat(self.history.clone(), Some(self.system_prompt.clone())).await?;

        // Add assistant response to history
        let assistant_msg = Message {
            role: Role::Assistant,
            content: response_text.clone(),
        };
        
        self.history.push(assistant_msg);

        Ok(response_text)
    }
}
