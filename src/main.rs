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

To create a tool, output a code block with language 'rhai' and the filename in a comment, like:
```rhai
// filename: my_tool
fn my_tool(args) {{
    return "result";
}}
```
To use a tool, use the format: [TOOL: tool_name(arg1, arg2)]
If you need to calculate something or get data, create a tool for it first.
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
