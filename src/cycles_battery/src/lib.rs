use candid::Deserialize;

use ic_cdk::{
    api::{self, management_canister::main::CanisterIdRecord, time},
    init,
};
use ic_cdk_timers::TimerId;
use ic_stable_structures::{writer::Writer, Memory as _, StableVec};

use std::cell::RefCell;

// Candid
use candid::{candid_method, Principal};

use ic_cdk::{post_upgrade, pre_upgrade, query, update};
use serde::Serialize;

mod guards;
use guards::assert_owner;

mod memory;
use memory::Memory;

mod datatype;
use datatype::{CyclesBalanceRecord, Timestamp, TopupRecord};

#[derive(Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub api_key: Option<String>,

    #[serde(skip, default = "init_stable_topup_store")]
    stable_topup_store: StableVec<TopupRecord, Memory>,

    #[serde(skip, default = "init_stable_cycles_monitor_store")]
    stable_cycles_monitor_store: StableVec<CyclesBalanceRecord, Memory>,
}

// constructor
#[init]
#[candid_method(init)]
fn init(owner: Option<Principal>, api_key: Option<String>) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            api_key: api_key,
            stable_topup_store: init_stable_topup_store(),
            stable_cycles_monitor_store: init_stable_cycles_monitor_store(),
        };
    });
}

// Public API
#[query()]
#[candid_method(query)]
fn get_public_cycles_balance() -> u64 {
    let cycles_balance = ic_cdk::api::canister_balance();
    cycles_balance
}

#[query(guard = "assert_owner")]
#[candid_method(query)]
fn get_cycles_balance() -> u64 {
    let cycles_balance = ic_cdk::api::canister_balance();
    cycles_balance
}

#[query()]
#[candid_method(query)]
fn get_owner() -> Principal {
    STATE.with(|s| s.borrow().owner.clone().unwrap())
}

// Permissioned API
#[query()]
#[candid_method(query)]
fn get_topup_records() -> Vec<TopupRecord> {
    STATE.with(|s| s.borrow().stable_topup_store.iter().collect())
}

#[query()]
#[candid_method(query)]
fn get_cycles_monitors() -> Vec<CyclesBalanceRecord> {
    STATE.with(|s| s.borrow().stable_cycles_monitor_store.iter().collect())
}

#[update()]
#[candid_method(update)]
fn log_cycles(group_name: String, api_key: String, cycles_balance: u64) -> Result<(), String> {
    // Check if the API key is correct
    if api_key != STATE.with(|s| s.borrow().api_key.clone()).unwrap() {
        return Err("Invalid API key".to_string());
    }

    // canister id from caller
    let caller_canister: Principal = api::caller();
    let caller_canister_id = caller_canister.to_text();

    let now: Timestamp = time();
    let record = CyclesBalanceRecord {
        canister_id: caller_canister_id,
        group_name: group_name,
        cycles_balance: cycles_balance,
        created_at: now,
    };

    let result = STATE.with(|s| s.borrow_mut().stable_cycles_monitor_store.push(&record));
    if result.is_err() {
        return Err("call to log_cycles push CyclesBalanceRecord failed".to_string());
    }

    return Ok(());
}

#[update()]
#[candid_method(update)]
async fn topup_cycles(
    group_name: String,
    api_key: String,
    req_cycles_amount: u128,
    cycles_balance: u64,
) -> Result<(), String> {
    // Check if the API key is correct
    if api_key != STATE.with(|s| s.borrow().api_key.clone()).unwrap() {
        return Err("Invalid API key".to_string());
    }

    // canister id from caller
    let caller_canister: Principal = api::caller();
    let caller_canister_id = caller_canister.to_text();

    // log topup record
    let now: Timestamp = time();
    let record = TopupRecord {
        canister_id: caller_canister_id.clone(),
        group_name: group_name.clone(),
        req_cycles_amount: req_cycles_amount,
        created_at: now,
    };

    let result = STATE.with(|s| s.borrow_mut().stable_topup_store.push(&record));
    if result.is_err() {
        return Err("call to topup_cycles push TopupRecord failed".to_string());
    }

    // log cycles balance
    let cycles_balance_record = CyclesBalanceRecord {
        canister_id: caller_canister_id.clone(),
        group_name: group_name.clone(),
        cycles_balance: cycles_balance,
        created_at: now,
    };

    let result = STATE.with(|s| {
        s.borrow_mut()
            .stable_cycles_monitor_store
            .push(&cycles_balance_record)
    });
    if result.is_err() {
        return Err("call to topup_cycles push CyclesBalanceRecord failed".to_string());
    }

    let canister_id_record = CanisterIdRecord {
        canister_id: caller_canister,
    };

    // transfer cycles to the canister with canister_id
    let ((),) = ic_cdk::api::call::call_with_payment128(
        Principal::management_canister(),
        "deposit_cycles",
        (canister_id_record,),
        req_cycles_amount,
    )
    .await
    .unwrap();

    return Ok(());
}

// Memory
impl Default for State {
    fn default() -> Self {
        Self {
            owner: None,
            api_key: None,
            stable_topup_store: init_stable_topup_store(),
            stable_cycles_monitor_store: init_stable_cycles_monitor_store(),
        }
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();

    /// The global vector to keep multiple timer IDs.
    static TIMER_IDS: RefCell<Vec<TimerId>> = RefCell::new(Vec::new());
}

fn init_stable_topup_store() -> StableVec<TopupRecord, Memory> {
    StableVec::init(memory::get_stable_topup_vec_memory())
        .expect("call to init_stable_topup_store fails")
}

fn init_stable_cycles_monitor_store() -> StableVec<CyclesBalanceRecord, Memory> {
    StableVec::init(memory::get_stable_cycles_monitor_vec_memory())
        .expect("call to init_stable_cycles_monitor_store fails")
}

// ---------------------- Canister upgrade process ----------------------
#[pre_upgrade]
fn pre_upgrade() {
    // Serialize the state.
    // This example is using CBOR, but you can use any data format you like.
    let mut state_bytes = vec![];
    STATE
        .with(|s| ciborium::ser::into_writer(&*s.borrow(), &mut state_bytes))
        .expect("failed to encode state");

    // Write the length of the serialized bytes to memory, followed by the
    // by the bytes themselves.
    let len = state_bytes.len() as u32;
    let mut memory = memory::get_upgrades_memory();
    let mut writer = Writer::new(&mut memory, 0);
    writer.write(&len.to_le_bytes()).unwrap();
    writer.write(&state_bytes).unwrap()
}

#[post_upgrade]
fn post_upgrade() {
    let memory = memory::get_upgrades_memory();

    // Read the length of the state bytes.
    let mut state_len_bytes = [0; 4];
    memory.read(0, &mut state_len_bytes);
    let state_len = u32::from_le_bytes(state_len_bytes) as usize;

    // Read the bytes
    let mut state_bytes = vec![0; state_len];
    memory.read(4, &mut state_bytes);

    // Deserialize and set the state.
    let state = ciborium::de::from_reader(&*state_bytes).expect("failed to decode state");
    STATE.with(|s| *s.borrow_mut() = state);
}

// ---------------------- Candid declarations did file generator ----------------------
#[cfg(test)]
mod tests {
    use crate::datatype::{CyclesBalanceRecord, TopupRecord};
    use candid::{export_service, Principal};

    #[test]
    fn save_candid() {
        use std::env;
        use std::fs::write;
        use std::path::PathBuf;

        let dir = PathBuf::from(env::current_dir().unwrap());
        export_service!();
        write(dir.join("cycles_battery.did"), __export_service()).expect("Write failed.");
    }
}
