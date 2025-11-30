use anyhow::Result;
use rust_research_agent::tools::ToolManager;
use std::time::Duration;

#[tokio::test]
async fn test_ipc_communication() -> Result<()> {
    let mut manager = ToolManager::new()?;
    
    // Start server on port 9999
    let result = manager.execute_tool("start_server", vec!["9999".to_string()])?;
    println!("Server start result: {}", result);
    assert!(result.contains("9999"));
    
    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Create a tool that calls send_message (workaround for 2-arg limitation)
    let send_tool_code = r#"
    fn test_send(dummy) {
        return send_message("http://127.0.0.1:9999/message", "Hello from test");
    }
    "#;
    manager.create_tool("test_send", send_tool_code)?;
    
    // Call the tool
    let message_result = manager.execute_tool("test_send", vec!["dummy".to_string()])?;
    
    println!("Send message result: {}", message_result);
    
    // Check if we got a response (should contain "ok" or "Response")
    assert!(message_result.contains("Response") || message_result.contains("ok"));

    Ok(())
}
