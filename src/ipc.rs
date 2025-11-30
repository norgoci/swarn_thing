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
}

impl IpcState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

async fn handle_message(
    State(state): State<IpcState>,
    Json(payload): Json<Message>,
) -> Json<MessageResponse> {
    println!("📨 Received message: {}", payload.content);
    
    // Store the message
    let mut messages = state.messages.lock().await;
    messages.push(payload.content.clone());
    
    Json(MessageResponse {
        status: "ok".to_string(),
        received: payload.content,
    })
}

pub async fn start_http_server(port: u16) -> Result<()> {
    let state = IpcState::new();
    
    let app = Router::new()
        .route("/message", post(handle_message))
        .with_state(state);
    
    let addr = format!("127.0.0.1:{}", port);
    println!("🚀 IPC Server starting on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
