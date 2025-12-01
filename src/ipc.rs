use anyhow::Result;
use axum::{
    extract::State,
    routing::post,
    Router,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::message::IpcMessage;
use crate::tools::PendingTool;
use std::sync::Mutex as StdMutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub status: String,
    pub received: String,
}

#[derive(Clone)]
pub struct IpcState {
    pub messages: Arc<Mutex<Vec<String>>>,
    pub pending_tools: Arc<StdMutex<Vec<PendingTool>>>,
}

impl IpcState {
    pub fn new(pending_tools: Arc<StdMutex<Vec<PendingTool>>>) -> Self {
        // Convert std::sync::Mutex to tokio::sync::Mutex for async usage if needed, 
        // or just wrap the std Mutex in Arc and use it.
        // Wait, PendingTool uses std::sync::Mutex in ToolManager.
        // Here we are in async context.
        // It's better to use std::sync::Mutex for shared data if critical sections are short.
        // But IpcState defines pending_tools.
        // ToolManager defines it as Arc<std::sync::Mutex<Vec<PendingTool>>>.
        // IpcState needs to match that type to share it.
        
        // Let's change IpcState definition to use std::sync::Mutex for pending_tools
        // to match ToolManager.
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            pending_tools,
        }
    }
}

async fn handle_message(
    State(state): State<IpcState>,
    Json(payload): Json<Message>,
) -> Json<MessageResponse> {
    // Try to parse as structured IpcMessage
    let ipc_msg = IpcMessage::from_json_or_text(&payload.content);
    
    let response_text = match ipc_msg {
        IpcMessage::ToolShare { name, code, description, safety_level } => {
            println!("📦 Received ToolShare: {} (Safety: {:?})", name, safety_level);
            
            // Add to pending queue
            let pending = PendingTool {
                name: name.clone(),
                code,
                source_agent: "remote_agent".to_string(), // In future, extract from request
                received_at: std::time::SystemTime::now(),
                description,
                safety_level,
            };
            
            if let Ok(mut tools) = state.pending_tools.lock() {
                tools.push(pending);
                format!("Tool '{}' received and queued for approval.", name)
            } else {
                "Error: Could not lock tool queue".to_string()
            }
        },
        IpcMessage::Text { content } => {
            println!("📨 Received message: {}", content);
            // Store the message
            let mut messages = state.messages.lock().await;
            messages.push(content.clone());
            content
        },
        IpcMessage::ToolRequest { name } => {
            println!("❓ Received request for tool: {}", name);
            format!("Request for '{}' received (auto-response not implemented)", name)
        }
    };
    
    Json(MessageResponse {
        status: "ok".to_string(),
        received: response_text,
    })
}

pub async fn start_http_server(port: u16, pending_tools: Arc<StdMutex<Vec<PendingTool>>>) -> Result<()> {
    let state = IpcState::new(pending_tools);
    
    let app = Router::new()
        .route("/message", post(handle_message))
        .with_state(state);
    
    let addr = format!("127.0.0.1:{}", port);
    println!("🚀 IPC Server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
