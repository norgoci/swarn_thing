use anyhow::Result;
use rust_research_agent::tools::ToolManager;

#[test]
fn test_tool_composition() -> Result<()> {
    let mut manager = ToolManager::new()?;
    
    // Create Tool A
    let code_a = r#"
    fn tool_a(x) {
        return x + "_A";
    }
    "#;
    manager.create_tool("tool_a", code_a)?;
    
    // Create Tool B that calls Tool A
    let code_b = r#"
    fn tool_b(x) {
        return tool_a(x) + "_B";
    }
    "#;
    manager.create_tool("tool_b", code_b)?;
    
    // Execute Tool B
    let result = manager.execute_tool("tool_b", vec!["test".to_string()])?;
    
    assert_eq!(result, "test_A_B");

    // Verify magic_math style tool (parsing int from string)
    let code_math = r#"
    fn magic_math(x) {
        let val = parse_int(x);
        return val * 2;
    }
    "#;
    manager.create_tool("magic_math", code_math)?;
    let result_math = manager.execute_tool("magic_math", vec!["10".to_string()])?;
    assert_eq!(result_math, "20");

    Ok(())
}
