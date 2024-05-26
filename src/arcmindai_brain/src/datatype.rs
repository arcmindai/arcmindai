use candid::Deserialize;
use serde;

pub type Timestamp = u64;
pub type Embeddings = Vec<f32>;

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResult {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<OpenAIResultChoices>,
    pub usage: OpenAIResultUsage,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultChoices {
    pub index: u8,
    pub message: OpenAIResultMessage,
    pub finish_reason: String,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultMessage {
    pub role: String,
    pub content: String,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultMessageContent {
    pub thoughts: OpenAIResultMessageContentThoughts,
    pub command: OpenAIResultMessageContentCommand,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultMessageContentThoughts {
    pub text: String,
    pub reasoning: String,
    pub plan: String,
    pub criticism: String,
    pub speak: String,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultMessageContentCommand {
    pub name: String,
    pub args: serde_json::Value,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultMessageContentCommandArgs {
    pub query: String,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIResultUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIEmbeddingResult {
    pub object: String,
    pub data: Vec<OpenAIEmbeddingData>,
    pub model: String,
    pub usage: OpenAIEmbeddingUsage,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIEmbeddingUsage {
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

#[derive(serde::Serialize, Deserialize)]
pub struct OpenAIEmbeddingData {
    pub object: String,
    pub index: u8,
    pub embedding: Embeddings,
}
