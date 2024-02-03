use candid::{CandidType, Decode, Deserialize, Encode};
use serde::Serialize;
use std::borrow::Cow;

// Stable Structures
use ic_stable_structures::{BoundedStorable, Storable};

const MAX_VALUE_SIZE: u32 = 1024 * 1024;

pub const VEC_SEARCH_TOP_K_NN: usize = 3;

pub const PROMPT_CMD_GOOGLE: &str = "google";
pub const PROMPT_CMD_BROWSE_WEBSITE: &str = "browse_website";
pub const PROMPT_CMD_START_AGENT: &str = "start_agent";
pub const PROMPT_CMD_WRITE_FILE_AND_SHUTDOWN: &str = "write_file_and_shutdown";
pub const PROMPT_CMD_DO_NOTHING: &str = "do_nothing";
pub const PROMPT_CMD_SHUTDOWN: &str = "shutdown";
pub const PROMPT_CMD_BEAMFI_STREAM_PAYMENT: &str = "beamfi_stream_payment";

pub const TOP_CMD_AGENT_NAME: &str = "ArcMind";
pub const TOP_CMD_AGENT_TASK: &str = "knowing the greatest knowledge of the world";

pub type Embeddings = Vec<f32>;

#[derive(CandidType, Deserialize, Serialize)]
pub enum VecQuery {
    Embeddings(Vec<f32>),
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct VecDoc {
    pub content: String,
    pub embeddings: Embeddings,
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct PlainDoc {
    pub content: String,
}

#[derive(Serialize)]
pub struct PromptContext {
    pub agent_name: String,
    pub agent_task: String,
    pub agent_goal: String,
    pub current_date_time: String,
    pub response_format: String,
    pub past_events: String,
}

#[derive(Serialize)]
pub struct WebQueryPromptContext {
    pub web_query: String,
    pub web_page_content: String,
}

#[derive(CandidType, Deserialize, PartialEq)]
pub enum GoalStatus {
    Scheduled,
    Running,
    Complete,
}

pub type Timestamp = u64;

#[derive(CandidType, Deserialize, PartialEq, Serialize)]
pub enum ChatRole {
    ArcMind,
    User,
    System,
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct ChatDisplayHistory {
    pub content: String,
    pub role: ChatRole,
    pub created_at_human: String,
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct ChatHistory {
    pub content: String,
    pub role: ChatRole,
    pub created_at: Timestamp,
}

impl Storable for ChatHistory {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for ChatHistory {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct PaymentTransaction {
    pub transaction_id: String,
    pub created_at: Timestamp,
}

impl Storable for PaymentTransaction {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for PaymentTransaction {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

// Goal Struct and Storable Trait
#[derive(CandidType, Deserialize)]
pub struct Goal {
    pub goal: String,
    pub result: Option<String>,
    pub status: GoalStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

impl Storable for Goal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for Goal {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

// HTTP
#[derive(CandidType, Deserialize, Clone)]
pub struct HeaderField(pub String, pub String);

#[derive(CandidType, Deserialize)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HeaderField>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(CandidType, Deserialize)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: Vec<HeaderField>,
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}
