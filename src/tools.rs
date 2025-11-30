use anyhow::{Result, anyhow};
use rhai::{Engine, Scope, AST};
use std::fs;
use std::path::PathBuf;

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

pub struct ToolManager {
    engine: Engine,
    global_ast: AST,
    tools_dir: PathBuf,
}

impl ToolManager {
    pub fn new() -> Result<Self> {
        let mut engine = Engine::new();
        let tools_dir = PathBuf::from("tools");
        
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

        engine.register_fn("start_server", |port: &str| -> String {
            let port_num: u16 = port.parse().unwrap_or(8080);
            
            println!("🚀 Starting IPC server on port {}", port_num);
            
            // Spawn server in background thread
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    if let Err(e) = crate::ipc::start_http_server(port_num).await {
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

        Ok(Self {
            engine,
            global_ast,
            tools_dir,
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
}
