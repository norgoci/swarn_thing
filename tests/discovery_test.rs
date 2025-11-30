use anyhow::Result;
use rust_research_agent::tools::ToolManager;

#[test]
fn test_tool_discovery() -> Result<()> {
    let mut manager = ToolManager::new()?;
    
    // Create a dummy tool to ensure list is not empty
    manager.create_tool("dummy_tool", r#"fn dummy_tool() { return "ok"; }"#)?;
    
    // Execute list_tools
    let result = manager.execute_tool("list_tools", vec![])?;
    
    println!("Discovery Result: {}", result);
    
    // Check if dummy_tool is in the list
    assert!(result.contains("dummy_tool"));
    
    // Check if magic_math (which exists in the repo) is in the list
    // Note: This depends on the actual file system state of the tools dir
    assert!(result.contains("magic_math"));

    Ok(())
}
