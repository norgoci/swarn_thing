use anyhow::{Result, anyhow};
use rhai::{Engine, Scope, AST};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use std::sync::{Arc, Mutex};
use crate::message::{ToolSafetyLevel, IpcMessage};

/// A tool awaiting approval before installation
#[derive(Debug, Clone)]
pub struct PendingTool {
    pub name: String,
    pub code: String,
    pub source_agent: String,
    pub received_at: SystemTime,
    pub description: Option<String>,
    pub safety_level: ToolSafetyLevel,
}

// Helper function for recursive directory copying
fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());
        
        if path.is_dir() {
            copy_dir_recursive(&path, &dest_path)?;
        } else {
            fs::copy(&path, &dest_path)?;
        }
    }
    
    Ok(())
}

fn validate_tool_code(code: &str) -> ToolSafetyLevel {
    // Basic validation logic
    if code.len() > 10_000 {
        return ToolSafetyLevel::HighRisk; // Too large
    }
    
    // Check for risky keywords
    if code.contains("write_file") || 
       code.contains("clone_agent") || 
       code.contains("start_server") ||
       code.contains("std::process") {
        return ToolSafetyLevel::HighRisk;
    }
    
    if code.contains("read_file") || code.contains("scrape_url") {
        return ToolSafetyLevel::MediumRisk;
    }
    
    if code.contains("send_message") {
        return ToolSafetyLevel::LowRisk;
    }
    
    // Default to Safe if just pure computation
    ToolSafetyLevel::Safe
}

pub struct ToolManager {
    engine: Engine,
    global_ast: AST,
    tools_dir: PathBuf,
    pub pending_tools: Arc<Mutex<Vec<PendingTool>>>,
}

impl ToolManager {
    pub fn new() -> Result<Self> {
        let mut engine = Engine::new();
        let tools_dir = PathBuf::from("tools");
        
        // Initialize pending tools early so it can be captured
        let pending_tools = Arc::new(Mutex::new(Vec::new()));
        
        if !tools_dir.exists() {
            fs::create_dir(&tools_dir)?;
        }

        // Register standard tools
        engine.register_fn("read_file", |path: &str| -> String {
            fs::read_to_string(path).unwrap_or_else(|e| format!("Error reading file: {}", e))
        });

        engine.register_fn("write_file", |path: &str, content: &str| -> String {
            fs::write(path, content).map(|_| "File written successfully".to_string())
                .unwrap_or_else(|e| format!("Error writing file: {}", e))
        });
        
        // Simple search mock (since implementing real search requires an API key)
        // In a real app, we'd use reqwest to call Google/Bing/SerpApi
        engine.register_fn("search", |query: &str| -> String {
            println!("Searching for: {}", query);
            format!("Mock search results for '{}': \n1. Rust is a systems programming language.\n2. Rhai is an embedded scripting language.", query)
        });

        // Real Web Scraper
        engine.register_fn("scrape_url", |url: &str| -> String {
            println!("Scraping URL: {}", url);
            // Note: In a real async app, we should use async reqwest, but Rhai functions are sync.
            // We use blocking reqwest here for simplicity in this demo, or spawn a thread.
            // For this MVP, we'll use std::process::Command to curl or just use blocking reqwest if enabled.
            // Since we didn't enable blocking feature, let's use a quick hack: spawn a runtime for this call.
            
            let url = url.to_string();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match reqwest::get(&url).await {
                        Ok(resp) => {
                            match resp.text().await {
                                Ok(text) => {
                                    let document = scraper::Html::parse_document(&text);
                                    let selector = scraper::Selector::parse("body").unwrap();
                                    if let Some(body) = document.select(&selector).next() {
                                        // Simple text extraction
                                        body.text().collect::<Vec<_>>().join(" ")
                                            .split_whitespace().take(200).collect::<Vec<_>>().join(" ") // Limit to 200 words
                                    } else {
                                        "No body found".to_string()
                                    }
                                },
                                Err(e) => format!("Error reading text: {}", e)
                            }
                        },
                        Err(e) => format!("Error fetching URL: {}", e)
                    }
                })
            }).join().unwrap_or_else(|_| "Thread panic".to_string())
        });

        // Tool Discovery
        let tools_dir_clone = tools_dir.clone();
        engine.register_fn("list_tools", move || -> String {
            let mut tools = Vec::new();
            if let Ok(entries) = fs::read_dir(&tools_dir_clone) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("rhai") {
                            if let Some(stem) = path.file_stem() {
                                tools.push(stem.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
            tools.join(", ")
        });

        // Tool Inspection
        let tools_dir_clone2 = tools_dir.clone();
        engine.register_fn("inspect_tool", move |tool_name: &str| -> String {
            let path = tools_dir_clone2.join(format!("{}.rhai", tool_name));
            match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => format!("Error: Tool '{}' not found", tool_name),
            }
        });

        // IPC Tools
        engine.register_fn("send_message", |url: &str, message: &str| -> String {
            println!("📤 Sending message to {}: {}", url, message);
            
            // Use blocking reqwest in a thread
            let url = url.to_string();
            let message = message.to_string();
            
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let client = reqwest::Client::new();
                    let payload = serde_json::json!({
                        "content": message
                    });
                    
                    match client.post(&url).json(&payload).send().await {
                        Ok(resp) => {
                            match resp.text().await {
                                Ok(text) => format!("Response: {}", text),
                                Err(e) => format!("Error reading response: {}", e),
                            }
                        },
                        Err(e) => format!("Error sending message: {}", e),
                    }
                })
            }).join().unwrap_or_else(|_| "Thread panic".to_string())
        });

        let pending_clone = pending_tools.clone();
        engine.register_fn("start_server", move |port: &str| -> String {
            let port_num: u16 = port.parse().unwrap_or(8080);
            let pending = pending_clone.clone();
            
            println!("🚀 Starting IPC server on port {}", port_num);
            
            // Spawn server in background thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = crate::ipc::start_http_server(port_num, pending).await {
                        eprintln!("Server error: {}", e);
                    }
                });
            });
            
            format!("IPC server starting on port {}", port_num)
        });

        // Self-Replication Tool
        engine.register_fn("clone_agent", |target_dir: &str| -> String {
            println!("🧬 Cloning agent to: {}", target_dir);
            
            // Create target directory
            if let Err(e) = fs::create_dir_all(target_dir) {
                return format!("Error creating directory: {}", e);
            }
            
            // 1. Copy executable
            match std::env::current_exe() {
                Ok(exe_path) => {
                    let exe_name = exe_path.file_name().unwrap_or_default();
                    let target_exe = PathBuf::from(target_dir).join(exe_name);
                    
                    if let Err(e) = fs::copy(&exe_path, &target_exe) {
                        return format!("Error copying executable: {}", e);
                    }
                    
                    // Make executable on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        if let Ok(metadata) = fs::metadata(&target_exe) {
                            let mut perms = metadata.permissions();
                            perms.set_mode(0o755);
                            let _ = fs::set_permissions(&target_exe, perms);
                        }
                    }
                },
                Err(e) => return format!("Error getting executable path: {}", e),
            }
            
            // 2. Copy tools directory
            let tools_src = PathBuf::from("tools");
            let tools_dst = PathBuf::from(target_dir).join("tools");
            
            if tools_src.exists() {
                if let Err(e) = copy_dir_recursive(&tools_src, &tools_dst) {
                    return format!("Error copying tools: {}", e);
                }
            }
            
            // 3. Copy .env if exists
            let env_src = PathBuf::from(".env");
            if env_src.exists() {
                let env_dst = PathBuf::from(target_dir).join(".env");
                let _ = fs::copy(&env_src, &env_dst);
            }
            
            format!("✅ Agent cloned successfully to: {}", target_dir)
        });

        // Initialize with an empty AST
        let global_ast = engine.compile("").map_err(|e| anyhow::anyhow!("Rhai init error: {}", e))?;

        // Register Pending Tool Management Functions
        
        // list_pending_tools
        let pending_clone = pending_tools.clone();
        engine.register_fn("list_pending_tools", move || -> String {
            let tools = pending_clone.lock().unwrap();
            if tools.is_empty() {
                return "No tools pending approval.".to_string();
            }
            
            let mut output = String::from("Pending Tools:\n");
            for (i, tool) in tools.iter().enumerate() {
                output.push_str(&format!("{}. {} (Safety: {:?}) - From: {}\n", 
                    i + 1, tool.name, tool.safety_level, tool.source_agent));
                if let Some(desc) = &tool.description {
                    output.push_str(&format!("   Description: {}\n", desc));
                }
            }
            output
        });

        // approve_tool
        let pending_clone = pending_tools.clone();
        let tools_dir_clone = tools_dir.clone();
        // Removed engine_clone as Engine is not Clone and we don't strictly need it for writing files
        // Actually Engine might not be cheap or thread safe to share like this for compilation inside closure?
        // Wait, create_tool logic needs to be duplicated or we need a way to call it.
        // create_tool modifies global_ast which is in ToolManager, not available here.
        // We can just write the file and let the next load pick it up? 
        // Or we can try to compile it here.
        // For MVP, let's just write the file and say "Installed. Restart or reload might be needed if hot reload not fully working".
        // But wait, create_tool in ToolManager does: write file + compile + merge AST.
        // We can't easily merge AST from here without access to ToolManager's global_ast.
        // However, we can register a function that just writes the file, and maybe we can trigger a reload?
        // Or we can rely on the fact that we are inside Rhai, maybe we can eval the code?
        // Let's just write the file for now. The agent might need to reload tools.
        // Actually, we can use the `engine` passed to `new`? No, we need to modify `global_ast` which is in `ToolManager`.
        // This is a limitation. 
        // Let's implement `approve_tool` to just write the file and return "Tool saved. Please run [TOOL: reload_tools()]" (if we had one).
        // Or better: The `ToolManager` methods I added (`approve_tool`) *do* have access to `self`.
        // But I can't call them from the registered function easily.
        // I will implement the logic to write file here.
        
        engine.register_fn("approve_tool", move |name: &str| -> String {
            let mut tools = pending_clone.lock().unwrap();
            if let Some(index) = tools.iter().position(|t| t.name == name) {
                let tool = tools.remove(index);
                let path = tools_dir_clone.join(format!("{}.rhai", tool.name));
                if let Err(e) = fs::write(&path, &tool.code) {
                    return format!("Error writing tool file: {}", e);
                }
                // We can't easily update global_ast here without shared access to it.
                // For Phase 1, we'll accept that it saves to disk. 
                // We can add a `reload_tools` native function later or just say it's available next run.
                // Actually, we can try to compile it using a temporary engine to check validity, but we can't add to global AST of the main engine easily from here.
                format!("Tool '{}' approved and saved to disk. It will be available after reload.", name)
            } else {
                format!("Tool '{}' not found in pending queue", name)
            }
        });

        // reject_tool
        let pending_clone = pending_tools.clone();
        engine.register_fn("reject_tool", move |name: &str| -> String {
            let mut tools = pending_clone.lock().unwrap();
            if let Some(index) = tools.iter().position(|t| t.name == name) {
                tools.remove(index);
                format!("Tool '{}' rejected and removed from queue", name)
            } else {
                format!("Tool '{}' not found in pending queue", name)
            }
        });
        
        // share_tool
        let tools_dir_clone = tools_dir.clone();
        engine.register_fn("share_tool", move |url: &str, tool_name: &str| -> String {
            // 1. Get tool code
            let path = tools_dir_clone.join(format!("{}.rhai", tool_name));
            let code = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => return format!("Error: Tool '{}' not found", tool_name),
            };
            
            // 2. Validate to get safety level
            // We need to duplicate validate_tool_code logic or make it available. 
            // It's a standalone function, so we can call it.
            // But it's defined below. We might need to move it up or use it.
            // Rust allows calling functions defined later.
            // But `validate_tool_code` is not in scope of the closure? It is if it's in the same module.
            // Wait, `validate_tool_code` is private. Closures in `new` can call private functions of the module.
            // But `validate_tool_code` returns `ToolSafetyLevel` which is imported.
            
            // We need to verify `validate_tool_code` is accessible.
            // It is defined in the same file.
            
            // 3. Create message
            // We need to determine safety level.
            // Let's assume we can call validate_tool_code.
            // Wait, I can't call a function inside the closure if it's not captured? 
            // No, static functions are fine.
            
            // However, `validate_tool_code` is defined *outside* `impl ToolManager`.
            // So it's just a function in the module.
            
            // We need to handle the async send inside sync closure.
            // Use the same thread spawn trick as send_message.
            
            let url = url.to_string();
            let tool_name = tool_name.to_string();
            let code_clone = code.clone();
            
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let safety = validate_tool_code(&code_clone);
                    
                    let msg = IpcMessage::tool_share(
                        &tool_name,
                        &code_clone,
                        Some("Shared via share_tool".to_string()),
                        safety
                    );
                    
                    let client = reqwest::Client::new();
                    match client.post(&url).json(&msg).send().await {
                        Ok(resp) => {
                            match resp.text().await {
                                Ok(text) => format!("Response: {}", text),
                                Err(e) => format!("Error reading response: {}", e),
                            }
                        },
                        Err(e) => format!("Error sending message: {}", e),
                    }
                })
            }).join().unwrap_or_else(|_| "Thread panic".to_string())
        });

        Ok(Self {
            engine,
            global_ast,
            tools_dir,
            pending_tools,
        })
    }

    pub fn load_tools(&mut self) -> Result<()> {
        // Load all .rhai files from tools directory
        for entry in fs::read_dir(&self.tools_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rhai") {
                let script = fs::read_to_string(&path)?;
                let ast = self.engine.compile(&script).map_err(|e| anyhow::anyhow!("Rhai compile error in {:?}: {}", path, e))?;
                self.global_ast += ast;
            }
        }
        Ok(())
    }

    pub fn create_tool(&mut self, name: &str, code: &str) -> Result<String> {
        let path = self.tools_dir.join(format!("{}.rhai", name));
        fs::write(&path, code)?;
        
        // Compile and merge immediately
        let ast = self.engine.compile(code).map_err(|e| anyhow::anyhow!("Rhai compile error: {}", e))?;
        self.global_ast += ast;
        
        Ok(format!("Tool '{}' created successfully at {:?}", name, path))
    }

    pub fn list_tools(&self) -> Vec<String> {
        // We can't easily list functions from AST in Rhai without iterating definitions, 
        // but for now we can just list files in the directory or keep a separate list if needed.
        // For this MVP, let's just list the files in the tools dir as the source of truth.
        let mut tools = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.tools_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("rhai") {
                        if let Some(stem) = path.file_stem() {
                            tools.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        tools
    }

    pub fn execute_tool(&self, name: &str, args: Vec<String>) -> Result<String> {
        let mut scope = Scope::new();
        
        // Handle arguments:
        // If the tool takes 1 arg, pass it directly.
        // If it takes 0, pass nothing.
        // If it takes >1, we might need to change main.rs or pass an array.
        // For now, we assume most tools take 1 string arg or 0.
        // If args is empty, call with ().
        // If args has 1 element, call with (arg,).
        
        let args_tuple = if args.is_empty() {
            rhai::Dynamic::from(())
        } else {
            rhai::Dynamic::from(args[0].clone())
        };

        // Try to call with global_ast (for script tools)
        // We need to handle the tuple conversion carefully. 
        // call_fn expects a tuple of arguments.
        // If we have 0 args, we pass ().
        // If we have 1 arg, we pass (arg,).
        
        let result: Result<rhai::Dynamic, _> = if args.is_empty() {
             self.engine.call_fn(&mut scope, &self.global_ast, name, ())
        } else {
             self.engine.call_fn(&mut scope, &self.global_ast, name, (args[0].clone(),))
        };

        match result {
            Ok(v) => Ok(v.to_string()),
            Err(e) => {
                // If function not found in AST, try native functions (empty AST)
                if e.to_string().contains("Function not found") {
                    // Try native functions using eval
                    let script = if args.is_empty() {
                        format!("{}()", name)
                    } else {
                        scope.push("arg0", args[0].clone());
                        format!("{}(arg0)", name)
                    };
                    
                    self.engine.eval_with_scope::<rhai::Dynamic>(&mut scope, &script)
                        .map(|v| v.to_string())
                        .map_err(|e2| anyhow!("Error executing tool '{}': {}", name, e2))
                } else {
                    Err(anyhow!("Error executing tool '{}': {}", name, e))
                }
            }
        }
    }

    pub fn queue_tool(&mut self, name: String, code: String, source_agent: String, description: Option<String>) -> Result<String> {
        let safety_level = validate_tool_code(&code);
        
        let pending = PendingTool {
            name: name.clone(),
            code,
            source_agent,
            received_at: SystemTime::now(),
            description,
            safety_level: safety_level.clone(),
        };
        
        self.pending_tools.lock().unwrap().push(pending);
        
        Ok(format!("Tool '{}' queued for approval (Safety: {:?})", name, safety_level))
    }

    pub fn approve_tool(&mut self, name: &str) -> Result<String> {
        let mut tools = self.pending_tools.lock().unwrap();
        if let Some(index) = tools.iter().position(|t| t.name == name) {
            let tool = tools.remove(index);
            // Drop lock before calling create_tool to avoid potential deadlocks (though create_tool doesn't lock pending_tools)
            drop(tools);
            self.create_tool(&tool.name, &tool.code)?;
            Ok(format!("Tool '{}' approved and installed successfully", name))
        } else {
            Err(anyhow!("Tool '{}' not found in pending queue", name))
        }
    }

    pub fn reject_tool(&mut self, name: &str) -> Result<String> {
        let mut tools = self.pending_tools.lock().unwrap();
        if let Some(index) = tools.iter().position(|t| t.name == name) {
            tools.remove(index);
            Ok(format!("Tool '{}' rejected and removed from queue", name))
        } else {
            Err(anyhow!("Tool '{}' not found in pending queue", name))
        }
    }

    pub fn list_pending_tools(&self) -> String {
        let tools = self.pending_tools.lock().unwrap();
        if tools.is_empty() {
            return "No tools pending approval.".to_string();
        }
        
        let mut output = String::from("Pending Tools:\n");
        for (i, tool) in tools.iter().enumerate() {
            output.push_str(&format!("{}. {} (Safety: {:?}) - From: {}\n", 
                i + 1, tool.name, tool.safety_level, tool.source_agent));
            if let Some(desc) = &tool.description {
                output.push_str(&format!("   Description: {}\n", desc));
            }
        }
        output
    }
}
