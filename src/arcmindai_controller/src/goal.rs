use candid::{CandidType, Decode, Deserialize, Encode};
use std::borrow::Cow;

// Stable Structures
use ic_stable_structures::{BoundedStorable, Storable};

const MAX_VALUE_SIZE: u32 = 1024 * 1024;

#[derive(CandidType, Deserialize, PartialEq)]
pub enum GoalStatus {
    Scheduled,
    Running,
    Complete,
}

pub type Timestamp = u64;

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
