use serde::{Deserialize, Serialize};

/// Safety classification for tools
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolSafetyLevel {
    Safe,       // Pure computation, no side effects
    LowRisk,    // Reads data, no writes
    MediumRisk, // Writes files, network calls
    HighRisk,   // System operations, cloning
}

/// IPC message types for inter-agent communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum IpcMessage {
    /// Plain text message (backward compatibility)
    Text { content: String },
    
    /// Tool sharing request
    ToolShare {
        name: String,
        code: String,
        description: Option<String>,
        safety_level: ToolSafetyLevel,
    },
    
    /// Request a specific tool from another agent
    ToolRequest { name: String },
}

impl IpcMessage {
    /// Create a text message
    pub fn text(content: impl Into<String>) -> Self {
        IpcMessage::Text {
            content: content.into(),
        }
    }
    
    /// Create a tool share message
    pub fn tool_share(
        name: impl Into<String>,
        code: impl Into<String>,
        description: Option<String>,
        safety_level: ToolSafetyLevel,
    ) -> Self {
        IpcMessage::ToolShare {
            name: name.into(),
            code: code.into(),
            description,
            safety_level,
        }
    }
    
    /// Create a tool request message
    pub fn tool_request(name: impl Into<String>) -> Self {
        IpcMessage::ToolRequest {
            name: name.into(),
        }
    }
    
    /// Try to parse from JSON, fallback to plain text
    pub fn from_json_or_text(json: &str) -> Self {
        serde_json::from_str(json).unwrap_or_else(|_| IpcMessage::text(json))
    }
    
    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_message() {
        let msg = IpcMessage::text("Hello");
        let json = msg.to_json().unwrap();
        let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
        
        match parsed {
            IpcMessage::Text { content } => assert_eq!(content, "Hello"),
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_tool_share_message() {
        let msg = IpcMessage::tool_share(
            "square",
            "fn square(x) { return x * x; }",
            Some("Squares a number".to_string()),
            ToolSafetyLevel::Safe,
        );
        
        let json = msg.to_json().unwrap();
        let parsed: IpcMessage = serde_json::from_str(&json).unwrap();
        
        match parsed {
            IpcMessage::ToolShare { name, code, safety_level, .. } => {
                assert_eq!(name, "square");
                assert!(code.contains("square"));
                assert_eq!(safety_level, ToolSafetyLevel::Safe);
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[test]
    fn test_backward_compatibility() {
        // Plain text should be parsed as Text message
        let msg = IpcMessage::from_json_or_text("Just a plain message");
        
        match msg {
            IpcMessage::Text { content } => assert_eq!(content, "Just a plain message"),
            _ => panic!("Should parse as Text"),
        }
    }
}
