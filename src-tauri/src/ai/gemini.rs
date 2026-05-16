use super::{ChatMessage, ContentBlock, Role, StreamEvent, ToolCall, ToolDef};
use crate::error::{AppError, Result};
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;

const SYSTEM_INSTRUCTION: &str = "You are Darric, an agent-first personal work tool. \
    You help the user manage their day — notes, meetings, tasks, and conversations. \
    Answer concisely and directly.";

#[derive(Clone)]
pub struct GeminiProvider {
    pub api_key: String,
    pub model: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            model: "gemini-2.0-flash".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        tx: &UnboundedSender<StreamEvent>,
    ) -> Result<()> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.model, self.api_key
        );

        let body = build_request(messages, tools);

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::Ai(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AppError::Ai(format!("Gemini {status}: {text}")));
        }

        parse_sse(response, tx).await
    }
}

fn build_request(messages: &[ChatMessage], tools: &[ToolDef]) -> Value {
    let contents: Vec<Value> = messages.iter().map(to_api_message).collect();

    let function_declarations: Vec<Value> = tools
        .iter()
        .map(|t| {
            let mut schema = t.input_schema.clone();
            // Gemini uses uppercase type names
            normalize_schema_types(&mut schema);
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": schema,
            })
        })
        .collect();

    let mut body = json!({
        "contents": contents,
        "systemInstruction": {
            "parts": [{"text": SYSTEM_INSTRUCTION}]
        },
        "generationConfig": {
            "maxOutputTokens": 8192,
        }
    });

    if !function_declarations.is_empty() {
        body["tools"] = json!([{"functionDeclarations": function_declarations}]);
    }

    body
}

fn normalize_schema_types(val: &mut Value) {
    if let Value::Object(map) = val {
        if let Some(t) = map.get_mut("type") {
            if let Some(s) = t.as_str() {
                *t = json!(s.to_uppercase());
            }
        }
        for v in map.values_mut() {
            normalize_schema_types(v);
        }
    } else if let Value::Array(arr) = val {
        for v in arr.iter_mut() {
            normalize_schema_types(v);
        }
    }
}

fn to_api_message(msg: &ChatMessage) -> Value {
    let role = match msg.role {
        Role::User => "user",
        Role::Assistant => "model",
    };

    let parts: Vec<Value> = msg
        .blocks
        .iter()
        .map(|b| match b {
            ContentBlock::Text(t) => json!({"text": t}),
            ContentBlock::ToolUse { name, input, .. } => {
                json!({"functionCall": {"name": name, "args": input}})
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
            } => {
                // tool_use_id is the function name for Gemini
                json!({
                    "functionResponse": {
                        "name": tool_use_id,
                        "response": {"content": content}
                    }
                })
            }
        })
        .collect();

    json!({"role": role, "parts": parts})
}

async fn parse_sse(
    response: reqwest::Response,
    tx: &UnboundedSender<StreamEvent>,
) -> Result<()> {
    let mut stream = response.bytes_stream();
    let mut buf = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Ai(e.to_string()))?;
        buf.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(end) = buf.find("\n\n") {
            let msg = buf[..end].to_string();
            buf = buf[end + 2..].to_string();
            process_sse_message(&msg, tx)?;
        }
    }

    tx.send(StreamEvent::Done).ok();
    Ok(())
}

fn process_sse_message(msg: &str, tx: &UnboundedSender<StreamEvent>) -> Result<()> {
    let mut data = "";

    for line in msg.lines() {
        if let Some(rest) = line.strip_prefix("data: ") {
            data = rest;
        }
    }

    if data.is_empty() || data == "[DONE]" {
        return Ok(());
    }

    let val: Value = serde_json::from_str(data)?;

    let Some(candidates) = val["candidates"].as_array() else {
        return Ok(());
    };

    for candidate in candidates {
        let Some(parts) = candidate["content"]["parts"].as_array() else {
            continue;
        };

        for part in parts {
            if let Some(text) = part["text"].as_str() {
                tx.send(StreamEvent::Delta(text.to_string())).ok();
            } else if let Some(fc) = part.get("functionCall") {
                let name = fc["name"].as_str().unwrap_or("").to_string();
                let input = fc["args"].clone();
                let id = uuid::Uuid::new_v4().to_string();
                tx.send(StreamEvent::ToolUse(ToolCall { id, name, input })).ok();
            }
        }
    }

    Ok(())
}
