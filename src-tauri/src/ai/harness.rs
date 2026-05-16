use super::{AiProvider, ChatMessage, ContentBlock, Role, StreamEvent, ToolCall};
use super::mcp::McpManager;
use crate::error::Result;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct AgentHarness {
    pub provider: Arc<AiProvider>,
    pub mcp: Arc<McpManager>,
}

impl AgentHarness {
    pub fn new(provider: AiProvider, mcp: Arc<McpManager>) -> Self {
        Self {
            provider: Arc::new(provider),
            mcp,
        }
    }

    /// Runs a full agent turn, streaming tokens to the frontend via Tauri events.
    /// Dispatches tool calls through MCP and loops until the model returns no more tool calls.
    /// Returns the full assistant text response.
    pub async fn run_turn(
        &self,
        user_message: &str,
        history: &mut Vec<ChatMessage>,
        handle: &AppHandle,
    ) -> Result<String> {
        history.push(ChatMessage::user(user_message));

        let mut full_text = String::new();

        loop {
            let tools = self.mcp.available_tools().await;
            let (tx, mut rx) = mpsc::unbounded_channel::<StreamEvent>();

            let provider = self.provider.clone();
            let messages_snap = history.clone();
            let tools_snap = tools.clone();

            // Spawn provider so we can read events concurrently (real streaming to frontend)
            let task = tokio::spawn(async move {
                provider.complete(&messages_snap, &tools_snap, &tx).await
            });

            let mut response_blocks: Vec<ContentBlock> = Vec::new();
            let mut current_text = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();

            while let Some(event) = rx.recv().await {
                match event {
                    StreamEvent::Delta(text) => {
                        current_text.push_str(&text);
                        full_text.push_str(&text);
                        handle.emit("chat_delta", &text).ok();
                    }
                    StreamEvent::ToolUse(call) => {
                        if !current_text.is_empty() {
                            response_blocks
                                .push(ContentBlock::Text(std::mem::take(&mut current_text)));
                        }
                        response_blocks.push(ContentBlock::ToolUse {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            input: call.input.clone(),
                        });
                        tool_calls.push(call);
                    }
                    StreamEvent::Done => break,
                }
            }

            // Propagate any provider error
            if let Ok(Err(e)) = task.await {
                return Err(e);
            }

            if !current_text.is_empty() {
                response_blocks.push(ContentBlock::Text(current_text));
            }

            history.push(ChatMessage {
                role: Role::Assistant,
                blocks: response_blocks,
            });

            if tool_calls.is_empty() {
                break;
            }

            // Execute tools and add results
            let mut result_blocks = Vec::new();
            for call in &tool_calls {
                let result = self
                    .mcp
                    .call_tool(&call.name, call.input.clone())
                    .await
                    .unwrap_or_else(|e| format!("Tool error: {e}"));

                result_blocks.push(ContentBlock::ToolResult {
                    tool_use_id: call.id.clone(),
                    content: result,
                });
            }

            history.push(ChatMessage {
                role: Role::User,
                blocks: result_blocks,
            });
        }

        Ok(full_text)
    }
}
