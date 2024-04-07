use candid::{CandidType, Decode, Deserialize, Encode};
use serde::Serialize;
use std::borrow::Cow;

// Stable Structures
use ic_stable_structures::{BoundedStorable, Storable};

const MAX_VALUE_SIZE: u32 = 1024 * 1024;

pub type Timestamp = u64;

#[derive(CandidType, Deserialize, Serialize)]
pub struct TopupRecord {
    pub canister_id: String,
    pub group_name: String,
    pub req_cycles_amount: u128,
    pub created_at: Timestamp,
}

impl Storable for TopupRecord {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for TopupRecord {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}

#[derive(CandidType, Deserialize, Serialize)]
pub struct CyclesBalanceRecord {
    pub canister_id: String,
    pub group_name: String,
    pub cycles_balance: u64,
    pub created_at: Timestamp,
}

impl Storable for CyclesBalanceRecord {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

impl BoundedStorable for CyclesBalanceRecord {
    const MAX_SIZE: u32 = MAX_VALUE_SIZE;
    const IS_FIXED_SIZE: bool = false;
}
