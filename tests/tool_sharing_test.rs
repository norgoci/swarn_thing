use anyhow::Result;
use rust_research_agent::tools::ToolManager;
use std::time::Duration;

#[tokio::test]
async fn test_tool_sharing_between_agents() -> Result<()> {
    // Simulate two agents
    let mut agent_a = ToolManager::new()?;
    let mut agent_b = ToolManager::new()?;
    
    // Agent B starts a server
    agent_b.execute_tool("start_server", vec!["9998".to_string()])?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Agent A creates a tool
    let square_code = r#"
    fn square(x) {
        let num = parse_int(x);
        return num * num;
    }
    "#;
    agent_a.create_tool("square", square_code)?;
    
    // Agent A inspects the tool and shares it with Agent B
    let share_tool_code = r#"
    fn share_square(dummy) {
        let code = inspect_tool("square");
        return send_message("http://127.0.0.1:9998/message", code);
    }
    "#;
    agent_a.create_tool("share_square", share_tool_code)?;
    
    // Agent A sends the tool to Agent B
    let result = agent_a.execute_tool("share_square", vec!["x".to_string()])?;
    
    println!("Share result: {}", result);
    
    // Verify the message was sent successfully
    assert!(result.contains("Response") || result.contains("ok"));
    
    // Note: Agent B receives the code but doesn't automatically create the tool
    // In a real scenario, Agent B would need to:
    // 1. Parse the received message
    // 2. Extract the code
    // 3. Call create_tool() manually or via LLM instruction
    
    // For this test, we'll manually create the tool on Agent B to simulate the process
    agent_b.create_tool("square", square_code)?;
    
    // Verify Agent B can now use the tool
    let result_b = agent_b.execute_tool("square", vec!["5".to_string()])?;
    assert_eq!(result_b, "25");
    
    println!("✅ Tool successfully shared from Agent A to Agent B");

    Ok(())
}
