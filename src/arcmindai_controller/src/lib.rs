use candid::Deserialize;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use time::format_description;
use time::OffsetDateTime;

// Stable Structures
use ic_cdk::api::time;
extern crate tinytemplate;

use ic_cdk_timers::TimerId;
use ic_stable_structures::{writer::Writer, Memory as _, StableVec};

mod datatype;
use datatype::{
    ChatHistory, ChatRole, Goal, GoalStatus, PromptContext, Timestamp, PROMPT_CMD_DO_NOTHING,
    PROMPT_CMD_INSERT_CHAT, PROMPT_CMD_SHUTDOWN, PROMPT_CMD_START_AGENT,
};

mod prompts;
use prompts::{PROMPT, RESPONSE_FORMAT};

use tinytemplate::TinyTemplate;

// Candid
use candid::{candid_method, Principal};

use ic_cdk::{
    api::{self},
    init, post_upgrade, pre_upgrade, query, update,
};
use serde::Serialize;

mod guards;
use guards::assert_owner;

mod memory;
use memory::Memory;

use async_recursion::async_recursion;

const MIN_INTERVAL_SECS: u64 = 10;

#[derive(Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub brain_canister: Option<Principal>,

    #[serde(skip, default = "init_stable_goal_data")]
    stable_goal_data: StableVec<Goal, Memory>,

    #[serde(skip, default = "init_stable_chathistory_data")]
    stable_chathistory_data: StableVec<ChatHistory, Memory>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: None,
            brain_canister: None,
            stable_goal_data: init_stable_goal_data(),
            stable_chathistory_data: init_stable_chathistory_data(),
        }
    }
}

// Mutable global state
thread_local! {
    static STATE: RefCell<State> = RefCell::default();

    /// The global vector to keep multiple timer IDs.
    static TIMER_IDS: RefCell<Vec<TimerId>> = RefCell::new(Vec::new());
}

fn init_stable_goal_data() -> StableVec<Goal, Memory> {
    StableVec::init(memory::get_stable_goal_vec_memory())
        .expect("call to init_stable_goal_data fails")
}

fn init_stable_chathistory_data() -> StableVec<ChatHistory, Memory> {
    StableVec::init(memory::get_stable_chathistory_vec_memory())
        .expect("call to init_stable_chathistory_data fails")
}

/// Initial canister balance to track the cycles usage.
static INITIAL_CANISTER_BALANCE: AtomicU64 = AtomicU64::new(0);
/// Canister cycles usage tracked in the periodic task.
static CYCLES_USED: AtomicU64 = AtomicU64::new(0);

// ---------------------- ArcMind AI Agent ----------------------
// entry function for user to ask questions
// TODO - add owner check back when full ArcMind AI is ready
#[update]
#[candid_method(update)]
async fn start_agent(question: String) -> String {
    let brain_canister: Principal = STATE.with(|state| (*state.borrow()).brain_canister.unwrap());
    let (result,): (String,) = ic_cdk::api::call::call(brain_canister, "ask", (question,))
        .await
        .expect("call to ask failed");

    return result;
}

async fn process_new_goals() {
    let len = STATE.with(|s| s.borrow().stable_goal_data.len());
    let mut i = 0;

    while i < len {
        let goal: Option<Goal> = STATE.with(|s| s.borrow().stable_goal_data.get(i));
        match goal {
            Some(my_goal) => {
                if my_goal.status == GoalStatus::Scheduled {
                    ic_cdk::println!("Processing Goal {}", i);

                    let question = my_goal.goal.clone();

                    let mut tt = TinyTemplate::new();
                    tt.add_template("prompt", PROMPT).unwrap();

                    let now_epoch: Timestamp = time();
                    let now =
                        OffsetDateTime::from_unix_timestamp_nanos(now_epoch.try_into().unwrap())
                            .unwrap();
                    let format =
                        format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
                            .unwrap();
                    let datetime_string = now.format(&format).unwrap();

                    let context = PromptContext {
                        goal: question,
                        current_date_time: datetime_string,
                        response_format: RESPONSE_FORMAT.to_string(),
                        past_events: "".to_string(),
                    };

                    let full_prompt = tt.render("prompt", &context).unwrap();
                    ic_cdk::println!("Initial Prompt:\n{}", full_prompt);

                    // update goal status to running to prevent duplicate processing
                    update_goal_status(i, my_goal, GoalStatus::Running);

                    // ------ Chain of Thoughts Main Loop ------
                    // TODO - create inital chain of throughts input, update prompts template to include agent_name, task, and prompt
                    run_chain_of_thoughts(i, full_prompt).await;
                }
            }
            None => {
                ic_cdk::println!("Goal not found: {}", i);
            }
        }

        i = i + 1;
    }

    track_cycles_used();
}

/*
 * Chain of Thoughts Main Loop
 * @param command: Chain of Thoughts response JSON string
 */
#[async_recursion]
async fn run_chain_of_thoughts(goal_key: u64, cof_result: String) -> String {
    // ------ Begin Chain of Thoughts ------

    // parse command string
    let cof_json = serde_json::from_str::<serde_json::Value>(&cof_result);
    if cof_json.is_err() {
        return "Invalid JSON response.".to_string();
    }

    let cof_json = cof_json.unwrap();
    let cof_cmd = cof_json["command"].clone();
    let cmd_name = cof_cmd["name"].as_str();

    // match and run command
    // TODO - add google, browse_website commands and update prompts template
    // TODO - implement delegate functions: google, browse_website
    match cmd_name {
        Some(PROMPT_CMD_INSERT_CHAT) => {
            let cmd_args = cof_cmd["args"].clone();
            let text = cmd_args["text"].as_str();
            if text.is_none() {
                return "Invalid insert_chat command.".to_string();
            }

            insert_chat(ChatRole::ArcMind, text.unwrap().to_string());

            // TODO - create next prompt
            let next_promot = "".to_string();

            return run_chain_of_thoughts(goal_key, next_promot).await;
        }
        Some(PROMPT_CMD_START_AGENT) => {
            let cmd_args = cof_cmd["args"].clone();
            let name = cmd_args["name"].as_str();
            // let task = cmd_args["task"].as_str();
            let prompt = cmd_args["prompt"].as_str();
            if name.is_none() {
                return "Invalid insert_chat command.".to_string();
            }

            let result: String = start_agent(prompt.unwrap().to_string()).await;

            // insert result into chat history
            insert_chat(ChatRole::ArcMind, result.clone());

            // TODO - create next prompt
            let next_promot = "".to_string();

            return run_chain_of_thoughts(goal_key, next_promot).await;
        }
        Some(PROMPT_CMD_DO_NOTHING) => {
            let result = "ArcMind AI has decided to do nothing".to_string();

            // insert result into chat history
            insert_chat(ChatRole::ArcMind, result.to_string());
            // save result
            save_result(goal_key, result.clone());

            return result;
        }
        Some(PROMPT_CMD_SHUTDOWN) => {
            // insert result into chat history
            insert_chat(ChatRole::ArcMind, cof_result.clone());
            // save result
            save_result(goal_key, cof_result.clone());

            return cof_result;
        }
        Some(n) => {
            let result = format!("ArcMind AI encountered an invalid command: {}", n);

            // insert result into chat history
            insert_chat(ChatRole::ArcMind, result.to_string());
            save_result(goal_key, result.clone());

            return result;
        }
        None => {
            let result = "ArcMind AI encountered no command.".to_string();

            // insert result into chat history
            insert_chat(ChatRole::ArcMind, result.to_string());
            save_result(goal_key, result.clone());

            return result;
        }
    }

    // ------ End of Chain of Thoughts ------
}

// Retrieves goal from stable data
// TODO - add owner check back when full ArcMind AI is ready
#[query]
#[candid_method(query)]
fn get_goal(key: u64) -> Option<Goal> {
    STATE.with(|s| s.borrow().stable_goal_data.get(key))
}

// Retrieves chathistory from stable data
// TODO - add owner check back when full ArcMind AI is ready
#[query]
#[candid_method(query)]
fn get_chathistory() -> Vec<ChatHistory> {
    STATE.with(|s| s.borrow().stable_chathistory_data.iter().collect())
}

// Inserts a goal into the stable data Goal Vec and ChatHistory Vec
// TODO - add owner check back when full ArcMind AI is ready
#[update]
#[candid_method(update)]
fn insert_goal(goal_string: String) {
    let now: Timestamp = time();
    let new_goal = Goal {
        goal: goal_string.clone(),
        status: GoalStatus::Scheduled,
        created_at: now,
        updated_at: now,
        result: None,
    };

    STATE.with(|s| {
        s.borrow_mut()
            .stable_goal_data
            .push(&new_goal)
            .expect("call to insert_goal failed")
    });

    insert_chat(ChatRole::User, goal_string.clone());
}

fn update_goal_status(index: u64, goal: Goal, status: GoalStatus) {
    let updated_goal: Goal = Goal {
        status: status,
        ..goal
    };
    STATE.with(|s| s.borrow_mut().stable_goal_data.set(index, &updated_goal));
}

// Complete a goal with result, called by controller itself
// TODO - remove candid method once main loop is implemented
#[update]
#[candid_method(update)]
fn save_result(key: u64, result: String) {
    let opt_goal: Option<Goal> = STATE.with(|s| s.borrow().stable_goal_data.get(key));

    match opt_goal {
        Some(my_goal) => {
            let now: Timestamp = time();
            let updated_goal: Goal = Goal {
                result: Some(result),
                status: GoalStatus::Complete,
                updated_at: now,
                ..my_goal
            };

            STATE.with(|s| s.borrow_mut().stable_goal_data.set(key, &updated_goal));
        }
        None => {
            ic_cdk::trap("Goal not found.");
        }
    }
}

// Insert chat, called by controller itself
fn insert_chat(role: ChatRole, content: String) {
    let now: Timestamp = time();
    let new_chat = ChatHistory {
        content: content,
        role: role,
        created_at: now,
    };

    STATE.with(|s| {
        s.borrow_mut()
            .stable_chathistory_data
            .push(&new_chat)
            .expect("call to insert_chat failed")
    });
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
            stable_goal_data: init_stable_goal_data(),
            stable_chathistory_data: init_stable_chathistory_data(),
        };
    });

    start_with_interval_secs(MIN_INTERVAL_SECS);
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

    // Start the periodic task
    start_with_interval_secs(MIN_INTERVAL_SECS);
}

// ---------------------- Periodic Task Timer ----------------------------------------------

#[update]
fn start_with_interval_secs(secs: u64) {
    let secs = Duration::from_secs(secs);
    ic_cdk::println!(
        "Controller canister: checking for new scheduled goal with {secs:?} interval..."
    );
    // Schedule a new periodic task to increment the counter.
    // ic_cdk_timers::set_timer_interval(secs, periodic_task);

    // To drive an async function to completion inside the timer handler,
    // use `ic_cdk::spawn()`, for example:
    let timer_id = ic_cdk_timers::set_timer_interval(secs, || ic_cdk::spawn(process_new_goals()));

    // Add the timer ID to the global vector.
    TIMER_IDS.with(|timer_ids| timer_ids.borrow_mut().push(timer_id));
}

// ---------------------- Cycles Usage Tracking  --------------------------------------
/// Tracks the amount of cycles used for the periodic task.
fn track_cycles_used() {
    // Update the `INITIAL_CANISTER_BALANCE` if needed.
    let current_canister_balance = ic_cdk::api::canister_balance();
    INITIAL_CANISTER_BALANCE.fetch_max(current_canister_balance, Ordering::Relaxed);
    // Store the difference between the initial and the current balance.
    let cycles_used = INITIAL_CANISTER_BALANCE.load(Ordering::Relaxed) - current_canister_balance;
    CYCLES_USED.store(cycles_used, Ordering::Relaxed);
}

/// Returns the amount of cycles used since the beginning of the execution.
///
/// Example usage: `dfx canister call timer cycles_used`
#[query]
fn cycles_used() -> u64 {
    CYCLES_USED.load(Ordering::Relaxed)
}

// ---------------------- Candid declarations did file generator ----------------------
#[cfg(test)]
mod tests {
    use crate::datatype::{ChatHistory, Goal};
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
