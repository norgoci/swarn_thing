use anyhow::Result;
use rust_research_agent::tools::ToolManager;

#[test]
fn test_tool_refinement() -> Result<()> {
    let mut manager = ToolManager::new()?;
    
    // 1. Create initial version of tool
    let code_v1 = r#"
    fn evolve_me() {
        return "version 1";
    }
    "#;
    manager.create_tool("evolve_me", code_v1)?;
    
    let result_v1 = manager.execute_tool("evolve_me", vec![])?;
    assert_eq!(result_v1, "version 1");

    // 2. Overwrite with version 2
    let code_v2 = r#"
    fn evolve_me() {
        return "version 2";
    }
    "#;
    manager.create_tool("evolve_me", code_v2)?;
    
    // 3. Execute again - should be version 2
    let result_v2 = manager.execute_tool("evolve_me", vec![])?;
    assert_eq!(result_v2, "version 2");

    Ok(())
}
