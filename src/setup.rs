use std::path::PathBuf;

pub fn run_setup() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Cannot determine home directory");
            std::process::exit(1);
        }
    };

    let binary = match std::env::current_exe() {
        Ok(path) => path.to_string_lossy().to_string(),
        Err(_) => {
            eprintln!("Cannot determine current binary path");
            std::process::exit(1);
        }
    };

    println!("Starting mem-nexus auto-setup...");

    // Core IDE injection
    let targets = vec![
        ("Cursor", home.join(".cursor/mcp.json"), true),
        ("Gemini CLI", home.join(".gemini/settings/mcp.json"), true),
        (
            "Antigravity",
            home.join(".gemini/antigravity/mcp_config.json"),
            true,
        ),
        (
            "Windsurf",
            home.join(".codeium/windsurf/mcp_config.json"),
            true,
        ),
        ("Claude Desktop", get_claude_desktop_path(&home), true),
        ("JetBrains", home.join(".jb-mcp.json"), true),
        ("Cline", get_cline_path(&home), true),
        ("Trae", home.join(".trae/mcp.json"), true),
        ("VS Code", get_vscode_mcp_path(&home), false), // Uses VS Code custom schema
    ];

    for (name, path, is_standard_mcp) in targets {
        if let Some(parent) = path.parent() {
            // Only inject if the IDE directory exists
            if !parent.exists() {
                continue;
            }

            if is_standard_mcp {
                if let Err(e) = write_standard_mcp_json(&path, &binary) {
                    println!("Failed to set up {}: {}", name, e);
                } else {
                    println!("Configured MCP Server in {} ({})", name, path.display());
                }
            } else {
                if let Err(e) = write_vscode_mcp_json(&path, &binary) {
                    println!("Failed to set up VS Code: {}", e);
                } else {
                    println!("Configured VS Code MCP in ({})", path.display());
                }
            }
        }
    }

    // Agent Rules Injection
    println!("\nInjecting behavioral rules...");
    crate::rules_inject::inject_all_rules(&home);

    println!("\n[mem-nexus] Setup Complete!");
}

fn write_standard_mcp_json(path: &std::path::Path, binary: &str) -> Result<(), String> {
    let mut json = if path.exists() {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = json.as_object_mut() {
        let servers = obj
            .entry("mcpServers")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(servers_obj) = servers.as_object_mut() {
            servers_obj.insert(
                "mem-nexus".to_string(),
                serde_json::json!({
                    "command": binary,
                    "args": []
                }),
            );
        }
    }

    let formatted = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    std::fs::write(path, formatted).map_err(|e| e.to_string())
}

fn write_vscode_mcp_json(path: &std::path::Path, binary: &str) -> Result<(), String> {
    let mut json = if path.exists() {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if let Some(obj) = json.as_object_mut() {
        let servers = obj
            .entry("servers")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(servers_obj) = servers.as_object_mut() {
            servers_obj.insert(
                "mem-nexus".to_string(),
                serde_json::json!({
                    "command": binary,
                    "args": []
                }),
            );
        }
    }

    let formatted = serde_json::to_string_pretty(&json).map_err(|e| e.to_string())?;
    std::fs::write(path, formatted).map_err(|e| e.to_string())
}

fn get_vscode_mcp_path(home: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/Code/User/mcp.json")
    }
    #[cfg(target_os = "linux")]
    {
        home.join(".config/Code/User/mcp.json")
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join("Code/User/mcp.json")
        } else {
            home.join(".config/Code/User/mcp.json")
        }
    }
}

fn get_claude_desktop_path(home: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/Claude/claude_desktop_config.json")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".claude_desktop_config.json")
    }
}

fn get_cline_path(home: &std::path::Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library/Application Support/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    }
    #[cfg(target_os = "linux")]
    {
        home.join(".config/Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json")
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            PathBuf::from(appdata).join(
                "Code/User/globalStorage/saoudrizwan.claude-dev/settings/cline_mcp_settings.json",
            )
        } else {
            home.join("cline_mcp_settings.json")
        }
    }
}
