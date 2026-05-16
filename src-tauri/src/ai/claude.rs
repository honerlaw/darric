use super::{ChatMessage, ContentBlock, Role, StreamEvent, ToolCall, ToolDef};
use crate::error::{AppError, Result};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

const SYSTEM_PROMPT: &str = "You are Darric, an agent-first personal work tool. \
    You help the user manage their day — notes, meetings, tasks, and conversations. \
    Answer concisely and directly.";

#[derive(Clone)]
pub struct ClaudeProvider {
    pub api_key: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        tx: &UnboundedSender<StreamEvent>,
    ) -> Result<()> {
        let body = build_request(messages, tools);

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Ai(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Ai(format!("Claude {status}: {text}")));
        }

        parse_sse(response, tx).await
    }
}

fn build_request(messages: &[ChatMessage], tools: &[ToolDef]) -> Value {
    let api_messages: Vec<Value> = messages.iter().map(to_api_message).collect();

    let tools_json: Vec<Value> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();

    let mut body = json!({
        "model": "claude-opus-4-5",
        "max_tokens": 8192,
        "stream": true,
        "system": SYSTEM_PROMPT,
        "messages": api_messages,
    });

    if !tools_json.is_empty() {
        body["tools"] = json!(tools_json);
    }

    body
}

fn to_api_message(msg: &ChatMessage) -> Value {
    let role = match msg.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };

    let content: Vec<Value> = msg
        .blocks
        .iter()
        .map(|b| match b {
            ContentBlock::Text(t) => json!({"type": "text", "text": t}),
            ContentBlock::ToolUse { id, name, input } => {
                json!({"type": "tool_use", "id": id, "name": name, "input": input})
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
            } => json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id,
                "content": content,
            }),
        })
        .collect();

    // Flatten single text block to a plain string for cleaner API calls
    if content.len() == 1 {
        if let Some(text) = content[0]["text"].as_str() {
            return json!({"role": role, "content": text});
        }
    }

    json!({"role": role, "content": content})
}

enum BlockState {
    Text,
    ToolUse {
        id: String,
        name: String,
        input_buf: String,
    },
}

async fn parse_sse(
    response: reqwest::Response,
    tx: &UnboundedSender<StreamEvent>,
) -> Result<()> {
    let mut stream = response.bytes_stream();
    let mut buf = String::new();
    let mut blocks: HashMap<usize, BlockState> = HashMap::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Ai(e.to_string()))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(end) = buf.find("\n\n") {
            let msg = buf[..end].to_string();
            buf = buf[end + 2..].to_string();
            process_sse_message(&msg, &mut blocks, tx)?;
        }
    }

    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn process_sse_message(
    msg: &str,
    blocks: &mut HashMap<usize, BlockState>,
    tx: &UnboundedSender<StreamEvent>,
) -> Result<()> {
    let mut data = "";
    let mut event_type = "";

    for line in msg.lines() {
        if let Some(rest) = line.strip_prefix("data: ") {
            data = rest;
        } else if let Some(rest) = line.strip_prefix("event: ") {
            event_type = rest;
        }
    }

    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }

    let val: Value = serde_json::from_str(data)?;

    match event_type {
        "content_block_start" => {
            let index = usize::try_from(val["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            match val["content_block"]["type"].as_str().unwrap_or("") {
                "text" => {
                    blocks.insert(index, BlockState::Text);
                }
                "tool_use" => {
                    let id = val["content_block"]["id"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let name = val["content_block"]["name"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    blocks.insert(
                        index,
                        BlockState::ToolUse {
                            id,
                            name,
                            input_buf: String::new(),
                        },
                    );
                }
                _ => {}
            }
        }

        "content_block_delta" => {
            let index = usize::try_from(val["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            match val["delta"]["type"].as_str().unwrap_or("") {
                "text_delta" => {
                    let text = val["delta"]["text"].as_str().unwrap_or("");
                    if matches!(blocks.get(&index), Some(BlockState::Text)) {
                        tx.send(StreamEvent::Delta(text.to_string())).ok();
                    }
                }
                "input_json_delta" => {
                    let partial = val["delta"]["partial_json"].as_str().unwrap_or("");
                    if let Some(BlockState::ToolUse { input_buf, .. }) = blocks.get_mut(&index) {
                        input_buf.push_str(partial);
                    }
                }
                _ => {}
            }
        }

        "content_block_stop" => {
            let index = usize::try_from(val["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            if let Some(BlockState::ToolUse { id, name, input_buf }) = blocks.remove(&index) {
                let input = serde_json::from_str::<Value>(&input_buf)
                    .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
                tx.send(StreamEvent::ToolUse(ToolCall { id, name, input })).ok();
            }
        }

        "message_stop" => {
            tx.send(StreamEvent::Done).ok();
        }

        _ => {}
    }

    Ok(())
}
