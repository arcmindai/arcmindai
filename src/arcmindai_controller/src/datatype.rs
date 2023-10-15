use candid::{CandidType, Decode, Deserialize, Encode};
use serde::Serialize;
use std::borrow::Cow;

// Stable Structures
use ic_stable_structures::{BoundedStorable, Storable};

const MAX_VALUE_SIZE: u32 = 1024 * 1024;

pub const PROMPT_CMD_GOOGLE: &str = "google";
pub const PROMPT_CMD_BROWSE_WEBSITE: &str = "browse_website";
pub const PROMPT_CMD_START_AGENT: &str = "start_agent";
pub const PROMPT_CMD_WRITE_FILE_AND_SHUTDOWN: &str = "write_file_and_shutdown";
pub const PROMPT_CMD_DO_NOTHING: &str = "do_nothing";
pub const PROMPT_CMD_SHUTDOWN: &str = "shutdown";

pub const TOP_CMD_AGENT_NAME: &str = "ArcMind";
pub const TOP_CMD_AGENT_TASK: &str = "knowing the greatest knowledge of the world";

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
