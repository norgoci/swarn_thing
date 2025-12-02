use anyhow::Result;
use dotenv::dotenv;
use std::io::{self, Write};
use text_colorizer::*;

use swarm_thing::agent::Agent;
use swarm_thing::tools::ToolManager;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    println!("{}", "Swarn Thing Initializing...".green().bold());

    let mut tool_manager = ToolManager::new()?;
    tool_manager.load_tools()?;
    let tools_list = tool_manager.list_tools().join(", ");
    println!(
        "Loaded {} tools: {}",
        tool_manager.list_tools().len(),
        tools_list
    );

    let system_prompt = format!(
        r#"You are a Research Agent powered by Rust.
You have the ability to create and use tools.
Available Tools: [{}]

IMPORTANT - Tool Reuse Policy:
1. BEFORE creating any new tool, check if an existing tool can fulfill the request
2. Use [TOOL: list_tools()] to see all available tools
3. Use [TOOL: inspect_tool(name)] to understand what a tool does
4. Consider composing multiple existing tools instead of creating a new one
5. ONLY create a new tool if no existing tool or combination can solve the task

Examples of Good Behavior:
- User asks "square of 11" and 'square' tool exists → Use [TOOL: square(11)] directly
- User asks "square and double" and 'double_square' exists → Use existing tool
- User asks "square and double" and only 'square' exists → Create a new tool that calls square()
- Only create new tools for genuinely new functionality

IMPORTANT - Rhai Scripting Limitations:
1. NO TUPLES: Rhai does not support tuples like `(a, b)`. Use arrays `[a, b]` or maps `#{a: 1, b: 2}` instead.
2. NO STRUCTS: You cannot define structs. Use object maps `#{ field: value }`.
3. RETURN VALUES: To return multiple values, return an array or object map.
4. PRINTING: Use `print()` or `debug()` for logging.

To create a tool (ONLY when necessary), output a code block with language 'rhai' and the filename in a comment:
```rhai
// filename: my_tool
fn my_tool(args) {{
    return "result";
}}
```

To use a tool, use the format: [TOOL: tool_name(arg1, arg2)]
If you need to calculate something or get data, check existing tools first, then create one if needed.
"#,
        tools_list
    );

    let mut agent = Agent::new(&system_prompt).await?;

    println!("{}", "Ready! Type 'exit' to quit.".green());

    loop {
        print!("{}", "> ".blue().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("exit") {
            break;
        }

        match agent.chat(input).await {
            Ok(response) => {
                println!("{}", response.cyan());

                // Simple parsing for tool creation (MVP)
                if response.contains("```rhai") {
                    // Extract code
                    let parts: Vec<&str> = response.split("```rhai").collect();
                    if let Some(code_part) = parts.get(1) {
                        if let Some(code) = code_part.split("```").next() {
                            // Extract filename from comment // filename: name
                            let name = code
                                .lines()
                                .find(|l| l.contains("// filename:"))
                                .map(|l| l.split(":").nth(1).unwrap_or("unknown").trim())
                                .unwrap_or("unknown_tool");

                            println!("{}", format!("Creating tool: {}", name).yellow());
                            match tool_manager.create_tool(name, code) {
                                Ok(msg) => println!("{}", msg.green()),
                                Err(e) => {
                                    println!("{}", format!("Error creating tool: {}", e).red())
                                }
                            }
                        }
                    }
                }

                // Simple parsing for tool execution
                if response.contains("[TOOL:") {
                    let start = response.find("[TOOL:").unwrap() + 7;
                    let end = response[start..].find("]").unwrap() + start;
                    let content = &response[start..end];
                    // content is like "name(args)"
                    if let Some(paren) = content.find('(') {
                        let name = &content[..paren];
                        let args_str = &content[paren + 1..content.len() - 1];
                        let args = vec![args_str.to_string()]; // Simplify args for now

                        println!("{}", format!("Executing tool: {}", name).yellow());
                        match tool_manager.execute_tool(name, args) {
                            Ok(res) => {
                                println!("{}", format!("Tool Output: {}", res).green());
                                // Feed back to agent? For now just print.
                            }
                            Err(e) => println!("{}", format!("Tool Error: {}", e).red()),
                        }
                    }
                }
            }
            Err(e) => println!("{}", format!("Error: {}", e).red()),
        }
    }

    Ok(())
}
