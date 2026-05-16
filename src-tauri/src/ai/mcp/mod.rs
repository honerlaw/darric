pub mod discovery;

use crate::error::{AppError, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::Mutex;

use super::ToolDef;

#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
}

struct McpProcess {
    _child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    tools: Vec<ToolDef>,
}

pub struct McpManager {
    configs: Vec<McpServerConfig>,
    processes: Mutex<HashMap<String, McpProcess>>,
}

impl McpManager {
    pub fn new(configs: Vec<McpServerConfig>) -> Self {
        Self {
            configs,
            processes: Mutex::new(HashMap::new()),
        }
    }

    pub fn server_names(&self) -> Vec<String> {
        self.configs.iter().map(|c| c.name.clone()).collect()
    }

    pub async fn available_tools(&self) -> Vec<ToolDef> {
        let mut all_tools = Vec::new();
        let mut procs = self.processes.lock().await;

        for config in &self.configs {
            match ensure_process(config, &mut procs).await {
                Ok(proc) => all_tools.extend(proc.tools.clone()),
                Err(e) => {
                    log::warn!("[mcp] failed to start {}: {e}", config.name);
                }
            }
        }

        all_tools
    }

    pub async fn call_tool(&self, tool_name: &str, input: Value) -> Result<String> {
        let mut procs = self.processes.lock().await;

        // Find which server owns this tool
        let server_name = self
            .configs
            .iter()
            .find(|c| {
                procs
                    .get(&c.name)
                    .is_some_and(|p| p.tools.iter().any(|t| t.name == tool_name))
            })
            .map(|c| c.name.clone())
            .ok_or_else(|| AppError::Ai(format!("no MCP server has tool '{tool_name}'")))?;

        let config = self
            .configs
            .iter()
            .find(|c| c.name == server_name)
            .ok_or_else(|| AppError::Ai("server config missing".to_string()))?;

        let proc = ensure_process(config, &mut procs).await?;

        let result = proc
            .send_request(
                "tools/call",
                json!({ "name": tool_name, "arguments": input }),
            )
            .await?;

        // Extract text content from result
        let content = result["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|item| item["text"].as_str())
            .unwrap_or("(no content)")
            .to_string();

        Ok(content)
    }
}

async fn ensure_process<'a>(
    config: &McpServerConfig,
    procs: &'a mut HashMap<String, McpProcess>,
) -> Result<&'a mut McpProcess> {
    if !procs.contains_key(&config.name) {
        let proc = spawn_process(config).await?;
        procs.insert(config.name.clone(), proc);
    }
    Ok(procs.get_mut(&config.name).expect("just inserted"))
}

async fn spawn_process(config: &McpServerConfig) -> Result<McpProcess> {
    let mut cmd = tokio::process::Command::new(&config.command);
    cmd.args(&config.args)
        .envs(&config.env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());

    let mut child = cmd.spawn()?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AppError::Ai("MCP stdin unavailable".to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Ai("MCP stdout unavailable".to_string()))?;
    let stdout = BufReader::new(stdout).lines();

    let mut proc = McpProcess {
        _child: child,
        stdin,
        stdout,
        next_id: 1,
        tools: Vec::new(),
    };

    // Initialize handshake
    proc.initialize().await?;

    // Fetch tools
    proc.tools = proc.fetch_tools().await.unwrap_or_default();

    log::info!(
        "[mcp] started '{}' with {} tool(s)",
        config.name,
        proc.tools.len()
    );

    Ok(proc)
}

impl McpProcess {
    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let mut line = serde_json::to_string(&request)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        // Read lines until we get our response (skip notifications)
        loop {
            let response_line = self
                .stdout
                .next_line()
                .await?
                .ok_or_else(|| AppError::Ai("MCP process closed stdout".to_string()))?;

            let response: Value = match serde_json::from_str(&response_line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if response["id"] == json!(id) {
                if let Some(err) = response.get("error") {
                    return Err(AppError::Ai(format!("MCP error: {err}")));
                }
                return Ok(response["result"].clone());
            }
        }
    }

    async fn initialize(&mut self) -> Result<()> {
        let _init_result = self
            .send_request(
                "initialize",
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "darric", "version": "0.1.0"}
                }),
            )
            .await?;

        // Send initialized notification (no response expected)
        let notif = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let mut line = serde_json::to_string(&notif)?;
        line.push('\n');
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn fetch_tools(&mut self) -> Result<Vec<ToolDef>> {
        let result = self.send_request("tools/list", json!({})).await?;

        let tools = result["tools"]
            .as_array()
            .ok_or_else(|| AppError::Ai("tools/list missing tools array".to_string()))?;

        let defs = tools
            .iter()
            .filter_map(|t| {
                let name = t["name"].as_str()?.to_string();
                let description = t["description"].as_str().unwrap_or("").to_string();
                let input_schema = t["inputSchema"].clone();
                Some(ToolDef {
                    name,
                    description,
                    input_schema,
                })
            })
            .collect();

        Ok(defs)
    }
}
