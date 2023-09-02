use candid::Deserialize;
use std::cell::RefCell;

// Stable Structures
use ic_stable_structures::{writer::Writer, Memory as _, StableVec};

mod memory;
use memory::Memory;

mod goal;
use goal::{Goal, GoalStatus};

// Candid
use candid::{candid_method, Principal};

use ic_cdk::{
    api::{self},
    init, post_upgrade, pre_upgrade, query, update,
};
use serde::Serialize;

use crate::guards::assert_owner;
mod guards;

#[derive(Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub brain_canister: Option<Principal>,

    #[serde(skip, default = "init_stable_data")]
    stable_data: StableVec<Goal, Memory>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: None,
            brain_canister: None,
            stable_data: init_stable_data(),
        }
    }
}

// Mutable global state
thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

fn init_stable_data() -> StableVec<Goal, Memory> {
    StableVec::init(crate::memory::get_stable_vec_memory()).expect("call to init_stable_data fails")
}

// ---------------------- ArcMind AI Agent ----------------------
// entry function for user to ask questions
#[update(guard = "assert_owner")]
#[candid_method(update)]
async fn ask(question: String) -> String {
    let brain_canister: Principal = STATE.with(|state| (*state.borrow()).brain_canister.unwrap());
    let (result,): (String,) = ic_cdk::api::call::call(brain_canister, "ask", (question,))
        .await
        .expect("call to ask failed");

    return result;
}

// Retrieves goal from stable data
#[query]
#[candid_method(query)]
fn get_goal(key: u64) -> Option<Goal> {
    STATE.with(|s| s.borrow().stable_data.get(key))
}

// Inserts a goal into the vector stable data
#[update(guard = "assert_owner")]
#[candid_method(update)]
fn insert_goal(value: Goal) {
    STATE.with(|s| {
        s.borrow_mut()
            .stable_data
            .push(&value)
            .expect("call to insert_goal failed")
    });
}

// Complete a goal with result
#[update]
#[candid_method(update)]
fn save_result(key: u64, result: String) {
    let my_goal: Goal = STATE.with(|s| s.borrow().stable_data.get(key)).unwrap();
    let updated_goal: Goal = Goal {
        result: Some(result),
        status: GoalStatus::Complete,
        ..my_goal
    };

    STATE.with(|s| s.borrow_mut().stable_data.set(key, &updated_goal));
}

// ---------------------- Supporting Functions ----------------------
// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(owner: Option<Principal>, brain_canister: Option<Principal>) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            brain_canister: brain_canister,
            stable_data: init_stable_data(),
        };
    });
}

#[query]
#[candid_method(query)]
pub fn get_owner() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).owner)
}

#[query]
#[candid_method(query)]
pub fn get_brain_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).brain_canister)
}

#[update(guard = "assert_owner")]
#[candid_method(update)]
pub fn update_owner(new_owner: Principal) {
    STATE.with(|state| {
        state.borrow_mut().owner = Some(new_owner);
    });
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
    use crate::goal::Goal;
    use candid::{export_service, Principal};

    #[test]
    fn save_candid() {
        use std::env;
        use std::fs::write;
        use std::path::PathBuf;

        let dir = PathBuf::from(env::current_dir().unwrap());
        export_service!();
        write(dir.join("arcmindai_controller.did"), __export_service()).expect("Write failed.");
    }
}
