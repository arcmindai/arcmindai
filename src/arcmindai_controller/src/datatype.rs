use candid::{CandidType, Decode, Deserialize, Encode};
use serde::Serialize;
use std::borrow::Cow;

// Stable Structures
use ic_stable_structures::{BoundedStorable, Storable};

const MAX_VALUE_SIZE: u32 = 1024 * 1024;

#[derive(Serialize)]
pub struct PromptContext {
    pub goal: String,
    pub current_date_time: String,
    pub response_format: String,
    pub past_events: String,
}

#[derive(CandidType, Deserialize, PartialEq)]
pub enum GoalStatus {
    Scheduled,
    Running,
    Complete,
}

pub type Timestamp = u64;

#[derive(CandidType, Deserialize, PartialEq)]
pub enum ChatRole {
    ArcMind,
    User,
    System,
}

#[derive(CandidType, Deserialize)]
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
