use anyhow::Result;
use rust_research_agent::tools::ToolManager;
use std::path::Path;

#[test]
fn test_agent_cloning() -> Result<()> {
    let mut manager = ToolManager::new()?;
    
    // Create a test tool to verify it gets copied
    let test_tool_code = r#"
    fn test_clone_tool(x) {
        return "I was cloned! " + x;
    }
    "#;
    manager.create_tool("test_clone_tool", test_tool_code)?;
    
    // Clone the agent to a temporary directory
    let clone_dir = "/tmp/rust_agent_clone_test";
    let result = manager.execute_tool("clone_agent", vec![clone_dir.to_string()])?;
    
    println!("Clone result: {}", result);
    
    // Verify the clone was successful
    assert!(result.contains("successfully") || result.contains("✅"));
    
    // Verify clone directory exists
    let clone_path = Path::new(clone_dir);
    assert!(clone_path.exists(), "Clone directory should exist");
    
    // Verify at least one file was copied (the executable)
    let entries: Vec<_> = std::fs::read_dir(clone_path)?.collect();
    assert!(!entries.is_empty(), "Clone directory should not be empty");
    
    // Verify tools directory exists
    let tools_path = Path::new(clone_dir).join("tools");
    assert!(tools_path.exists(), "Tools directory should exist in clone");
    
    // Verify the test tool was copied
    let test_tool_path = tools_path.join("test_clone_tool.rhai");
    assert!(test_tool_path.exists(), "Test tool should be copied to clone");
    
    // Verify magic_math (original tool) was also copied
    let magic_math_path = tools_path.join("magic_math.rhai");
    assert!(magic_math_path.exists(), "Original tools should be copied");
    
    // Clean up
    std::fs::remove_dir_all(clone_dir)?;
    
    println!("✅ Agent cloning test passed!");

    Ok(())
}
