use anyhow::Result;
use rust_research_agent::tools::ToolManager;

#[test]
fn test_tool_inspection() -> Result<()> {
    let mut manager = ToolManager::new()?;
    
    // Create a tool with known content
    let code = r#"
    fn secret_logic(x) {
        return x + " is secret";
    }
    "#;
    manager.create_tool("secret_logic", code)?;
    
    // Inspect the tool
    let result = manager.execute_tool("inspect_tool", vec!["secret_logic".to_string()])?;
    
    println!("Inspection Result:\n{}", result);
    
    // Check if the source code is returned
    assert!(result.contains("fn secret_logic"));
    assert!(result.contains("is secret"));
    
    // Test non-existent tool
    let result_missing = manager.execute_tool("inspect_tool", vec!["nonexistent".to_string()])?;
    assert!(result_missing.contains("not found"));

    Ok(())
}
