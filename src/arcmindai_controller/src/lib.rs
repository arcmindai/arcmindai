use candid::Deserialize;
use serde_json::json;
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
    ChatHistory, ChatRole, Goal, GoalStatus, PromptContext, Timestamp, PROMPT_CMD_BROWSE_WEBSITE,
    PROMPT_CMD_DO_NOTHING, PROMPT_CMD_GOOGLE, PROMPT_CMD_SHUTDOWN, PROMPT_CMD_START_AGENT,
    PROMPT_CMD_WRITE_FILE,
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

use datatype::{RECENT_CHAT_HISTORY, TOP_CMD_AGENT_NAME, TOP_CMD_AGENT_TASK};

const MIN_INTERVAL_SECS: u64 = 10;

#[derive(Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub brain_canister: Option<Principal>,
    pub tools_canister: Option<Principal>,

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
            tools_canister: None,
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

                    // update goal status to running to prevent duplicate processing
                    update_goal_status(i, my_goal, GoalStatus::Running);

                    // ------ Chain of Thoughts Main Loop ------
                    let cof_input = create_cof_command(question.clone());
                    run_chain_of_thoughts(i, cof_input.to_string(), question.to_string()).await;
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

fn create_cof_command(prompt: String) -> String {
    let cof_input = json!({
      "command": {
          "name": "start_agent",
          "args": {
              "name": TOP_CMD_AGENT_NAME,
              "task": TOP_CMD_AGENT_TASK,
              "prompt": prompt
          }
      }
    });
    return cof_input.to_string();
}

fn create_prompt(
    agent_name: String,
    agent_task: String,
    agent_goal: String,
    history: Vec<ChatHistory>,
) -> String {
    let mut tt = TinyTemplate::new();
    let template_name = "prompt";
    tt.add_template(template_name, PROMPT).unwrap();

    let now_epoch: Timestamp = time();
    let now = OffsetDateTime::from_unix_timestamp_nanos(now_epoch.try_into().unwrap()).unwrap();
    let format =
        format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();
    let datetime_string = now.format(&format).unwrap();

    // Serialize most recent history to JSON string.
    let recent_history = history
        .iter()
        .rev()
        .take(RECENT_CHAT_HISTORY)
        .collect::<Vec<_>>();
    let past_events = serde_json::to_string(&recent_history).unwrap();

    ic_cdk::println!("past_events length: {}", past_events.len());

    let context = PromptContext {
        agent_name: agent_name,
        agent_task: agent_task,
        agent_goal: agent_goal,
        current_date_time: datetime_string,
        response_format: RESPONSE_FORMAT.to_string(),
        past_events: past_events.to_string(),
    };

    let full_prompt = tt.render(template_name, &context).unwrap();
    ic_cdk::println!("full_prompt: {}", full_prompt);

    return full_prompt;
}

/*
 * Chain of Thoughts Main Loop
 * @param command: Chain of Thoughts response JSON string
 */
#[async_recursion]
async fn run_chain_of_thoughts(goal_key: u64, cof_input: String, main_goal: String) -> String {
    // ------ Begin Chain of Thoughts ------

    // parse command string
    let cof_json = serde_json::from_str::<serde_json::Value>(&cof_input);
    if cof_json.is_err() {
        return "Invalid JSON response.".to_string();
    }

    let cof_json = cof_json.unwrap();
    let cof_cmd = cof_json["command"].clone();
    let cmd_name = cof_cmd["name"].as_str();

    // match and run command
    match cmd_name {
        Some(PROMPT_CMD_START_AGENT) => {
            let cmd_args = cof_cmd["args"].clone();
            let name = cmd_args["name"].as_str();
            let task = cmd_args["task"].as_str();
            let prompt = cmd_args["prompt"].as_str();
            if name.is_none() || task.is_none() || prompt.is_none() {
                return "Invalid insert_chat command.".to_string();
            }

            // create full prompt
            let full_prompt = create_prompt(
                name.unwrap().to_string(),
                task.unwrap().to_string(),
                prompt.unwrap().to_string(),
                get_chathistory(),
            );

            // insert result into chat history
            let result: String = start_agent(full_prompt).await;
            insert_chat(ChatRole::ArcMind, result.clone());

            return run_chain_of_thoughts(goal_key, result, main_goal.to_string()).await;
        }
        Some(PROMPT_CMD_GOOGLE) => {
            let cmd_args = cof_cmd["args"].clone();
            let query = cmd_args["query"].as_str();
            if query.is_none() {
                return "Invalid google command.".to_string();
            }

            let result: String = google(query.unwrap().to_string()).await;

            // insert result into chat history
            insert_chat(ChatRole::ArcMind, result.clone());

            let google_cmd_history = "Command google returned: Result saved successfully.";
            insert_chat(ChatRole::System, google_cmd_history.to_string());

            let generate_next_cmd = "GENERATE NEXT COMMAND JSON";
            insert_chat(ChatRole::User, generate_next_cmd.to_string());

            let next_command = create_cof_command(main_goal.to_string());
            return run_chain_of_thoughts(goal_key, next_command, main_goal.to_string()).await;
        }
        Some(PROMPT_CMD_BROWSE_WEBSITE) => {
            let cmd_args = cof_cmd["args"].clone();
            let url = cmd_args["url"].as_str();
            let question: Option<&str> = cmd_args["question"].as_str();
            if url.is_none() || question.is_none() {
                return "Invalid browse_website command.".to_string();
            }

            let result: String =
                browse_website(url.unwrap().to_string(), question.unwrap().to_string()).await;

            // insert result into chat history
            insert_chat(ChatRole::System, result.clone());

            let browse_website_cmd_history =
                "Command browse_website returned -> Result saved successfully.";
            insert_chat(ChatRole::System, browse_website_cmd_history.to_string());

            let generate_next_cmd = "GENERATE NEXT COMMAND JSON";
            insert_chat(ChatRole::User, generate_next_cmd.to_string());

            let next_command = create_cof_command(main_goal.to_string());
            return run_chain_of_thoughts(goal_key, next_command, main_goal.to_string()).await;
        }
        Some(PROMPT_CMD_WRITE_FILE) => {
            let cmd_args = cof_cmd["args"].clone();
            let key = cmd_args["key"].as_str();
            let text = cmd_args["text"].as_str();
            if text.is_none() || key.is_none() {
                return "Invalid write_file command.".to_string();
            }

            write_file(key.unwrap().to_string(), text.unwrap().to_string());

            let write_cmd_history = "Command write_to_file returned: File written to successfully.";
            insert_chat(ChatRole::System, write_cmd_history.to_string());

            let generate_next_cmd = "GENERATE NEXT COMMAND JSON";
            insert_chat(ChatRole::User, generate_next_cmd.to_string());

            let next_command: String = create_cof_command(main_goal.to_string());
            return run_chain_of_thoughts(goal_key, next_command, main_goal.to_string()).await;
        }
        Some(PROMPT_CMD_DO_NOTHING) => {
            // insert result into chat history
            let result = "ArcMind AI has decided to do nothing. End of processing.".to_string();
            insert_chat(ChatRole::System, result.to_string());
            // save result
            save_result(goal_key, result.clone());

            return result;
        }
        Some(PROMPT_CMD_SHUTDOWN) => {
            // insert result into chat history
            insert_chat(ChatRole::System, cof_input.clone());
            // save result
            save_result(goal_key, cof_input.clone());
            // insert shutdown result into chat history
            let shutdown_result =
                "ArcMind AI has completed the goal. End of processing.".to_string();
            insert_chat(ChatRole::System, shutdown_result.to_string());

            return cof_input;
        }
        Some(n) => {
            // insert result into chat history
            let result = format!(
                "ArcMind AI encountered an invalid command stopped processing: {}",
                n
            );
            insert_chat(ChatRole::System, result.to_string());
            save_result(goal_key, result.clone());

            return result;
        }
        None => {
            // insert result into chat history
            let result = "ArcMind AI encountered no command and stopped processing.".to_string();
            insert_chat(ChatRole::System, result.to_string());
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

fn write_file(_key: String, text: String) {
    insert_chat(ChatRole::ArcMind, text);
}

async fn google(query: String) -> String {
    let tools_canister: Principal = STATE.with(|state| (*state.borrow()).tools_canister.unwrap());
    let (result,): (String,) = ic_cdk::api::call::call(tools_canister, "google", (query,))
        .await
        .expect("call to google failed");

    return result;
}

async fn browse_website(url: String, _question: String) -> String {
    let tools_canister: Principal = STATE.with(|state| (*state.borrow()).tools_canister.unwrap());
    let (result,): (String,) = ic_cdk::api::call::call(tools_canister, "browse_website", (url,))
        .await
        .expect("call to browse_website failed");

    return result;
}

#[update]
#[candid_method(update)]
fn start_new_goal() {
    // clear and reinit stable_chathistory_data and stable_goal_data
    STATE.with(|s| {
        s.borrow_mut().stable_chathistory_data =
            StableVec::new(memory::get_stable_chathistory_vec_memory())
                .expect("call to get_stable_goal_vec_memory fails");
        s.borrow_mut().stable_goal_data = StableVec::new(memory::get_stable_goal_vec_memory())
            .expect("call to get_stable_goal_vec_memory fails")
    });
}

// ---------------------- Supporting Functions ----------------------
// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(
    owner: Option<Principal>,
    brain_canister: Option<Principal>,
    tools_canister: Option<Principal>,
) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            brain_canister: brain_canister,
            tools_canister: tools_canister,
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

#[query]
#[candid_method(query)]
pub fn get_tools_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).tools_canister)
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
