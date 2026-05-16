pub mod claude;
pub mod gemini;
pub mod harness;
pub mod mcp;

use serde_json::Value;

#[derive(Debug, Clone)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub enum ContentBlock {
    Text(String),
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub blocks: Vec<ContentBlock>,
}

impl ChatMessage {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            blocks: vec![ContentBlock::Text(text.into())],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

pub enum StreamEvent {
    Delta(String),
    ToolUse(ToolCall),
    Done,
}

pub enum AiProvider {
    Claude(claude::ClaudeProvider),
    Gemini(gemini::GeminiProvider),
}

impl AiProvider {
    pub async fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        tx: &tokio::sync::mpsc::UnboundedSender<StreamEvent>,
    ) -> crate::error::Result<()> {
        match self {
            Self::Claude(p) => p.complete(messages, tools, tx).await,
            Self::Gemini(p) => p.complete(messages, tools, tx).await,
        }
    }
}
