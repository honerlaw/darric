use super::McpServerConfig;
use std::collections::HashMap;

/// Reads ~/Library/Application Support/Claude/claude_desktop_config.json and returns
/// all configured MCP servers. Returns an empty list if the file is absent or unparseable.
pub fn load_claude_desktop_configs() -> Vec<McpServerConfig> {
    let path = dirs_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        log::debug!("[mcp] Claude Desktop config not found at {}", path.display());
        return Vec::new();
    };

    let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
        log::warn!("[mcp] failed to parse Claude Desktop config");
        return Vec::new();
    };

    let Some(servers) = json["mcpServers"].as_object() else {
        return Vec::new();
    };

    let mut configs = Vec::new();

    for (name, def) in servers {
        let Some(command) = def["command"].as_str() else {
            continue;
        };

        let args: Vec<String> = def["args"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();

        let env: HashMap<String, String> = def["env"]
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        configs.push(McpServerConfig {
            name: name.clone(),
            command: command.to_string(),
            args,
            env,
        });
    }

    log::info!("[mcp] discovered {} server(s) from Claude Desktop", configs.len());
    configs
}

fn dirs_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
    std::path::PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json")
}
