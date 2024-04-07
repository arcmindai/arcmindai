use candid::Deserialize;
use plugin_types::AMPluginAction;
use serde_json::json;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use time::format_description;
use time::OffsetDateTime;

use ic_cdk::api::time;
use ic_cdk_timers::TimerId;
use ic_stable_structures::{writer::Writer, Memory as _, StableVec};

mod datatype;
use datatype::{
    ChatDisplayHistory, ChatHistory, ChatRole, Embeddings, Goal, GoalStatus, HttpRequest,
    HttpResponse, PaymentTransaction, PlainDoc, PromptContext, Timestamp, VecDoc, VecQuery,
    WebQueryPromptContext, PROMPT_CMD_BEAMFI_STREAM_PAYMENT, PROMPT_CMD_BROWSE_WEBSITE,
    PROMPT_CMD_DO_NOTHING, PROMPT_CMD_GOOGLE, PROMPT_CMD_SHUTDOWN, PROMPT_CMD_START_AGENT,
    PROMPT_CMD_WRITE_FILE_AND_SHUTDOWN, TOP_CMD_AGENT_NAME, TOP_CMD_AGENT_TASK,
    VEC_SEARCH_TOP_K_NN,
};

mod prompts;
use prompts::{COF_PROMPT, RESPONSE_FORMAT, WEB_QUERY_PROMPT};

extern crate tinytemplate;
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

mod beamfi_stream;
use beamfi_stream::BeamFiPlugin;

pub mod plugin_types;

use async_recursion::async_recursion;

use crate::datatype::get_path;

// 3 secs
const NEW_GOALS_CHECK_MIN_INTERVAL_SECS: u64 = 3;
// 3 days
const CYCLES_BALANCE_CHECK_MIN_INTERVAL_SECS: u64 = 60 * 60 * 24 * 3;
// Cycle usage threshold
const CYCLES_ONE_TC: u64 = 1_000_000_000_000;
const CYCLES_THRESHOLD: u64 = 3 * CYCLES_ONE_TC;
const CYCLES_TOPUP_AMT: u64 = 4 * CYCLES_ONE_TC;

const CYCLES_TOPUP_GROUP: &str = "arcmindai_controller";

const RECENT_CHAT_HISTORY: usize = 40;
const DATE_TIME_FORMAT: &str = "[year]-[month]-[day] [hour]:[minute]:[second]";
const MAX_NUM_COF_PER_GOAL: u16 = 60;
const DEFAULT_MAX_NUM_THOUGHTS_ALLOWED: u16 = 500;

#[derive(Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,

    pub brain_canister: Option<Principal>,
    pub tools_canister: Option<Principal>,
    pub vector_canister: Option<Principal>,
    pub beamfi_canister: Option<Principal>,
    pub battery_canister: Option<Principal>,

    pub is_pause_chain_of_thoughts: Option<bool>,
    pub browse_website_gpt_model: Option<String>,
    pub battery_api_key: Option<String>,

    pub max_num_thoughts_allowed: u64,
    pub num_thoughts_processed: u64,
    pub billing_key: Option<String>,

    #[serde(skip, default = "init_stable_goal_data")]
    stable_goal_data: StableVec<Goal, Memory>,

    #[serde(skip, default = "init_stable_chathistory_data")]
    stable_chathistory_data: StableVec<ChatHistory, Memory>,

    #[serde(skip, default = "init_stable_paymenttransaction_data")]
    stable_paymenttransaction_data: StableVec<PaymentTransaction, Memory>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            owner: None,
            brain_canister: None,
            tools_canister: None,
            vector_canister: None,
            beamfi_canister: None,
            battery_canister: None,
            is_pause_chain_of_thoughts: Some(false),
            browse_website_gpt_model: None,
            battery_api_key: None,
            max_num_thoughts_allowed: DEFAULT_MAX_NUM_THOUGHTS_ALLOWED as u64,
            num_thoughts_processed: 0,
            billing_key: None,
            stable_goal_data: init_stable_goal_data(),
            stable_chathistory_data: init_stable_chathistory_data(),
            stable_paymenttransaction_data: init_stable_paymenttransaction_data(),
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

fn init_stable_paymenttransaction_data() -> StableVec<PaymentTransaction, Memory> {
    StableVec::init(memory::get_stable_paymenttransaction_vec_memory())
        .expect("call to init_stable_paymenttransaction_data fails")
}

/// Initial canister balance to track the cycles usage.
static INITIAL_CANISTER_BALANCE: AtomicU64 = AtomicU64::new(0);
/// Canister cycles usage tracked in the periodic task.
static CYCLES_USED: AtomicU64 = AtomicU64::new(0);

// ---------------------- ArcMind AI Agent ----------------------
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
                    run_chain_of_thoughts(0, i, cof_input.to_string(), question.to_string()).await;
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

//  Check if the cycles balance is below the threshold, and topup from Cycles Battery canister if necessary
#[update]
#[candid_method(update)]
async fn check_cycles_and_topup() {
    // Get the cycles balance
    let current_canister_balance = ic_cdk::api::canister_balance();

    // log the cycles balance
    ic_cdk::println!("Current canister balance: {}", current_canister_balance);

    let battery_api_key: Option<String> =
        STATE.with(|state| (*state.borrow()).battery_api_key.clone());
    let battery_canister = STATE.with(|state| (*state.borrow()).battery_canister.unwrap());

    // Make Topup request if the balance is below the threshold
    if current_canister_balance < CYCLES_THRESHOLD {
        ic_cdk::println!("Cycles balance is below the threshold");

        let cycles_topup: u64 = CYCLES_TOPUP_AMT;
        // convert cycles_topup to u128
        let cycles_topup_input: u128 = cycles_topup as u128;

        let (result,): (Result<(), String>,) = ic_cdk::api::call::call(
            battery_canister.clone(),
            "topup_cycles",
            (
                CYCLES_TOPUP_GROUP,
                battery_api_key.unwrap(),
                cycles_topup_input,
                current_canister_balance,
            ),
        )
        .await
        .expect("call to ask failed");

        if result.is_ok() {
            ic_cdk::println!("Cycles balance topped up by {}", cycles_topup);
        } else {
            ic_cdk::println!("Cycles balance topup failed: {}", result.unwrap_err());
        }
    } else {
        ic_cdk::println!("Cycles balance is above the threshold");

        let (result,): (Result<(), String>,) = ic_cdk::api::call::call(
            battery_canister.clone(),
            "log_cycles",
            (
                CYCLES_TOPUP_GROUP,
                battery_api_key.unwrap(),
                current_canister_balance,
            ),
        )
        .await
        .expect("call to ask failed");

        if result.is_ok() {
            ic_cdk::println!("Cycles balance logged: {}", current_canister_balance);
        } else {
            ic_cdk::println!("Cycles balance log failed: {}", result.unwrap_err());
        }
    }
}

fn create_cof_command(prompt: String) -> String {
    let cof_input = json!({
      "command": {
          "name": PROMPT_CMD_START_AGENT,
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
    top_lt_memory: Option<Vec<PlainDoc>>,
) -> String {
    let mut tt = TinyTemplate::new();
    let template_name = "prompt";
    tt.add_template(template_name, COF_PROMPT).unwrap();

    let format_desc = format_description::parse(DATE_TIME_FORMAT).unwrap();

    // iterate through history, truncate content to 1000 chars
    let mut recent_display_history: Vec<ChatDisplayHistory> = Vec::new();
    let mut i = 0;

    for chat in history {
        if i >= RECENT_CHAT_HISTORY {
            break;
        }

        let created_at_dt =
            OffsetDateTime::from_unix_timestamp_nanos(chat.created_at.try_into().unwrap()).unwrap();
        let created_at_human = created_at_dt.format(&format_desc).unwrap();
        let chat_display = ChatDisplayHistory {
            content: chat.content,
            role: chat.role,
            created_at_human: created_at_human,
        };

        recent_display_history.push(chat_display);
        i += 1;
    }

    for doc in top_lt_memory.unwrap() {
        let chat_display = ChatDisplayHistory {
            content: doc.content,
            role: ChatRole::System,
            created_at_human: "".to_string(),
        };

        recent_display_history.push(chat_display);
    }

    let past_events = serde_json::to_string(&recent_display_history).unwrap();

    let now_epoch: Timestamp = time();
    let now = OffsetDateTime::from_unix_timestamp_nanos(now_epoch.try_into().unwrap()).unwrap();
    let current_datetime_string = now.format(&format_desc).unwrap();

    let context = PromptContext {
        agent_name: agent_name,
        agent_task: agent_task,
        agent_goal: agent_goal,
        current_date_time: current_datetime_string,
        response_format: RESPONSE_FORMAT.to_string(),
        past_events: past_events.to_string(),
    };

    let full_prompt = tt.render(template_name, &context).unwrap();
    ic_cdk::println!("full_prompt: {}", full_prompt);

    return full_prompt;
}

fn create_web_query_prompt(query: String, content: String) -> String {
    let mut tt = TinyTemplate::new();
    let template_name = "web_query_prompt";
    tt.add_template(template_name, WEB_QUERY_PROMPT).unwrap();

    let context = WebQueryPromptContext {
        web_query: query,
        web_page_content: content,
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
async fn run_chain_of_thoughts(
    num_thoughts: u16,
    goal_key: u64,
    cof_input: String,
    main_goal: String,
) -> String {
    // ------ Begin Chain of Thoughts ------
    let is_pause_chain_of_thoughts: bool =
        STATE.with(|state| (*state.borrow()).is_pause_chain_of_thoughts.unwrap());
    if is_pause_chain_of_thoughts {
        let message = "Chain of Thoughts is paused.".to_string();
        insert_chat(ChatRole::System, message.clone());
        return message.clone();
    }

    if num_thoughts >= MAX_NUM_COF_PER_GOAL {
        let message = "Chain of Thoughts has reached max number of thoughts per goal.".to_string();
        insert_chat(ChatRole::System, message.clone());
        return message.clone();
    }

    if is_exceed_max_num_thoughts_allowed() {
        let message: String =
            "Chain of Thoughts has reached max number of thoughts allowed for the plan."
                .to_string();
        insert_chat(ChatRole::System, message.clone());
        return message.clone();
    }

    // parse command string
    let cof_json = serde_json::from_str::<serde_json::Value>(&cof_input);
    if cof_json.is_err() {
        insert_chat(
            ChatRole::System,
            "ArcMind AI encountered invalid JSON response from previous command. A recovery commnand would be sent.".to_string(),
        );
        return run_recovery_cmd(num_thoughts, goal_key, main_goal).await;
    }

    let cof_json = cof_json.unwrap();
    let cof_cmd = cof_json["command"].clone();
    let cmd_name = cof_cmd["name"].as_str();

    inc_num_thoughts_processed();

    // match and run command
    match cmd_name {
        Some(PROMPT_CMD_START_AGENT) => {
            let cmd_args = cof_cmd["args"].clone();
            let name = cmd_args["name"].as_str();
            let task = cmd_args["task"].as_str();
            let prompt = cmd_args["prompt"].as_str();

            if name.is_none() || task.is_none() || prompt.is_none() {
                let sys_result =
                    format!("ArcMind AI encountered an invalid command: {}", cof_input);
                insert_chat(ChatRole::System, sys_result.to_string());

                let user_result =
                    "The command you provided is invalid. Use a valid command and try again.";
                insert_chat(ChatRole::User, user_result.to_string());

                let next_command = create_cof_command(main_goal.to_string());
                return run_chain_of_thoughts(
                    num_thoughts + 1,
                    goal_key,
                    next_command,
                    main_goal.to_string(),
                )
                .await;
            }

            // get the first chatdisplayhistory from recent_display_history
            let recent_display_history = get_chathistory();
            let first_chat_display_history = recent_display_history.first().unwrap();

            // generate embeddings using first_chat_display_history.content
            let embeddings: Embeddings =
                generate_embeddings(first_chat_display_history.content.clone())
                    .await
                    .unwrap();

            // load relevant long term memory from vector_db canister
            let top_lt_memory: Option<Vec<PlainDoc>> = search_vecdoc(embeddings).await;

            // create full prompt
            let full_prompt = create_prompt(
                name.unwrap().to_string(),
                task.unwrap().to_string(),
                prompt.unwrap().to_string(),
                recent_display_history,
                top_lt_memory,
            );

            // insert result into chat history
            let result: String = start_agent(full_prompt, None).await;
            insert_chat(ChatRole::ArcMind, result.clone());

            return run_chain_of_thoughts(
                num_thoughts + 1,
                goal_key,
                result,
                main_goal.to_string(),
            )
            .await;
        }
        Some(PROMPT_CMD_GOOGLE) => {
            let cmd_args = cof_cmd["args"].clone();
            let query = cmd_args["query"].as_str();
            if query.is_none() {
                return "Invalid google command.".to_string();
            }

            let result: String = google(query.unwrap().to_string()).await;

            // insert result into chat history
            insert_chat(ChatRole::System, result.clone());

            let google_cmd_history = "Command google returned: Result saved successfully.";
            insert_chat(ChatRole::System, google_cmd_history.to_string());

            // generate embeddings
            let embeddings: Embeddings = generate_embeddings(result.clone()).await.unwrap();

            // save embeddings to vectordb
            add_vecdoc(result.clone(), embeddings).await;

            let next_command = create_cof_command(main_goal.to_string());
            return run_chain_of_thoughts(
                num_thoughts + 1,
                goal_key,
                next_command,
                main_goal.to_string(),
            )
            .await;
        }
        Some(PROMPT_CMD_BROWSE_WEBSITE) => {
            let cmd_args = cof_cmd["args"].clone();
            let url = cmd_args["url"].as_str();
            let question: Option<&str> = cmd_args["question"].as_str();
            if url.is_none() || question.is_none() {
                return "Invalid browse_website command.".to_string();
            }

            let web_page_content: String =
                browse_website(url.unwrap().to_string(), question.unwrap().to_string()).await;

            // create web query prompt
            let web_query_prompt =
                create_web_query_prompt(question.unwrap().to_string(), web_page_content);

            // extract browse_website_gpt_model from state
            let gpt_model: Option<String> =
                STATE.with(|state| (*state.borrow()).browse_website_gpt_model.clone());

            let result: String = start_agent(web_query_prompt, gpt_model).await;
            insert_chat(ChatRole::System, result.clone());

            let browse_website_cmd_history =
                "Command browse_website returned -> Result saved successfully.";
            insert_chat(ChatRole::System, browse_website_cmd_history.to_string());

            // generate embeddings
            let embeddings: Embeddings = generate_embeddings(result.clone()).await.unwrap();

            // save embeddings to vectordb
            add_vecdoc(result.clone(), embeddings).await;

            let next_command = create_cof_command(main_goal.to_string());
            return run_chain_of_thoughts(
                num_thoughts + 1,
                goal_key,
                next_command,
                main_goal.to_string(),
            )
            .await;
        }
        Some(PROMPT_CMD_WRITE_FILE_AND_SHUTDOWN) => {
            let cmd_args = cof_cmd["args"].clone();
            let key = cmd_args["key"].as_str();
            let text = cmd_args["text"].as_str();
            if text.is_none() || key.is_none() {
                return "Invalid write_file_and_shutdown command.".to_string();
            }

            write_file_and_shutdown(key.unwrap().to_string(), text.unwrap().to_string());

            let write_cmd_history = "Command write_file_and_shutdown has run successfully.";
            insert_chat(ChatRole::System, write_cmd_history.to_string());

            // insert shutdown result into chat history
            let shutdown_result =
                "ArcMind AI has completed the goal. End of processing.".to_string();
            insert_chat(ChatRole::System, shutdown_result.to_string());

            return cof_input;
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
            // save result
            save_result(goal_key, cof_input.clone());

            // insert shutdown result into chat history
            let shutdown_result =
                "ArcMind AI has completed the goal. End of processing.".to_string();
            insert_chat(ChatRole::System, shutdown_result.to_string());

            return cof_input;
        }
        Some(PROMPT_CMD_BEAMFI_STREAM_PAYMENT) => {
            let cmd_args = cof_cmd["args"].clone();
            let amount = cmd_args["amount"].as_str();
            let token_type = cmd_args["token_type"].as_str();
            let recipient_principa_id = cmd_args["recipient_principal_id"].as_str();

            if amount.is_none() || token_type.is_none() || recipient_principa_id.is_none() {
                return "Invalid beanfi stream command.".to_string();
            }

            let args = vec![
                amount.unwrap().to_string(),
                token_type.unwrap().to_string(),
                recipient_principa_id.unwrap().to_string(),
            ];
            let beamfi_plugin: BeamFiPlugin = AMPluginAction::new();
            let beamfi_canister: Principal =
                STATE.with(|state| (*state.borrow()).beamfi_canister.unwrap());
            let controller_canister: Principal = api::id();

            let escrow_id = beamfi_plugin
                .invoke(controller_canister, beamfi_canister, args)
                .await;

            ic_cdk::println!("BeamFi streaming escrow_id {}", escrow_id);

            insert_chat(
                ChatRole::System,
                "Command beamfi_stream_payment has executed successfully.".to_string(),
            );

            insert_chat(
                ChatRole::System,
                "Please move on to the next command. If none is left, please shutdown.".to_string(),
            );

            let next_command = create_cof_command(main_goal.to_string());
            return run_chain_of_thoughts(
                num_thoughts + 1,
                goal_key,
                next_command,
                main_goal.to_string(),
            )
            .await;
        }
        Some(n) => {
            insert_chat(
                ChatRole::System,
                format!("ArcMind AI encountered an invalid command: {}", n),
            );
            return run_recovery_cmd(num_thoughts, goal_key, main_goal).await;
        }
        None => {
            insert_chat(
                ChatRole::System,
                "ArcMind AI encountered None command".to_string(),
            );
            return run_recovery_cmd(num_thoughts, goal_key, main_goal).await;
        }
    }

    // ------ End of Chain of Thoughts ------
}

fn inc_num_thoughts_processed() {
    STATE.with(|state| {
        let cur_state = state.borrow().num_thoughts_processed;
        state.borrow_mut().num_thoughts_processed = cur_state + 1;
    });
}

async fn run_recovery_cmd(num_thoughts: u16, goal_key: u64, main_goal: String) -> String {
    let user_result = "The command you provided is invalid. Use a valid command and try again.";
    insert_chat(ChatRole::User, user_result.to_string());

    let next_command = create_cof_command(main_goal.to_string());
    return run_chain_of_thoughts(
        num_thoughts + 1,
        goal_key,
        next_command,
        main_goal.to_string(),
    )
    .await;
}

async fn start_agent(question: String, gpt_model: Option<String>) -> String {
    let brain_canister: Principal = STATE.with(|state| (*state.borrow()).brain_canister.unwrap());
    let num_retries: i8 = 0;
    let (result,): (String,) =
        ic_cdk::api::call::call(brain_canister, "ask", (question, gpt_model, num_retries))
            .await
            .expect("call to ask failed");

    return result;
}

async fn add_vecdoc(content: String, embeddings: Embeddings) -> String {
    let vector_canister: Principal = STATE.with(|state| (*state.borrow()).vector_canister.unwrap());

    let vec_doc = VecDoc {
        content: content.clone(),
        embeddings: embeddings.clone(),
    };

    let (result,): (String,) = ic_cdk::api::call::call(vector_canister, "add", (vec_doc,))
        .await
        .expect("call to vector_canister.add failed");

    return result;
}

async fn search_vecdoc(embeddings: Embeddings) -> Option<Vec<PlainDoc>> {
    let query: VecQuery = VecQuery::Embeddings(embeddings);
    let vector_canister: Principal = STATE.with(|state| (*state.borrow()).vector_canister.unwrap());

    let (result,): (Option<Vec<PlainDoc>>,) =
        ic_cdk::api::call::call(vector_canister, "search", (query, VEC_SEARCH_TOP_K_NN))
            .await
            .expect("call to vector_canister.search failed");

    return result;
}

async fn generate_embeddings(content: String) -> Result<Embeddings, String> {
    let brain_canister: Principal = STATE.with(|state| (*state.borrow()).brain_canister.unwrap());
    let num_retries: i8 = 0;
    let (result,): (Result<Embeddings, String>,) = ic_cdk::api::call::call(
        brain_canister,
        "generate_embeddings",
        (content, num_retries),
    )
    .await
    .expect("call to generate_embeddings failed");

    return result;
}

fn get_paymenttransction() -> Vec<PaymentTransaction> {
    STATE.with(|s| s.borrow().stable_paymenttransaction_data.iter().collect())
}

// Retrieves goal from stable data
#[query(guard = "assert_owner")]
#[candid_method(query)]
fn get_goal(key: u64) -> Option<Goal> {
    STATE.with(|s| s.borrow().stable_goal_data.get(key))
}

// Retrieves chathistory from stable data
#[query(guard = "assert_owner")]
#[candid_method(query)]
fn get_chathistory() -> Vec<ChatHistory> {
    STATE.with(|s| s.borrow().stable_chathistory_data.iter().collect())
}

// Inserts a goal into the stable data Goal Vec and ChatHistory Vec
#[update(guard = "assert_owner")]
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

// Inserts a goal into the stable data Goal Vec and ChatHistory Vec, and clear existing goals
#[update(guard = "assert_owner")]
#[candid_method(update)]
fn start_new_goal(goal_string: String) {
    let now: Timestamp = time();
    let new_goal = Goal {
        goal: goal_string.clone(),
        status: GoalStatus::Scheduled,
        created_at: now,
        updated_at: now,
        result: None,
    };

    clear_all_goals();

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

fn write_file_and_shutdown(_key: String, text: String) {
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

#[update(guard = "assert_owner")]
#[candid_method(update)]
fn clear_all_goals() {
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
    vector_canister: Option<Principal>,
    beamfi_canister: Option<Principal>,
    battery_canister: Option<Principal>,
    browse_website_gpt_model: Option<String>,
    billing_key: Option<String>,
    battery_api_key: Option<String>,
) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            brain_canister: brain_canister,
            tools_canister: tools_canister,
            vector_canister: vector_canister,
            beamfi_canister: beamfi_canister,
            battery_canister: battery_canister,
            is_pause_chain_of_thoughts: Some(false),
            browse_website_gpt_model: browse_website_gpt_model,
            battery_api_key: battery_api_key,
            max_num_thoughts_allowed: DEFAULT_MAX_NUM_THOUGHTS_ALLOWED as u64,
            num_thoughts_processed: 0,
            billing_key: billing_key,
            stable_goal_data: init_stable_goal_data(),
            stable_chathistory_data: init_stable_chathistory_data(),
            stable_paymenttransaction_data: init_stable_paymenttransaction_data(),
        };
    });

    // Start the periodic tasks
    start_new_goals_check_timer(NEW_GOALS_CHECK_MIN_INTERVAL_SECS);
    start_cycles_check_timer(CYCLES_BALANCE_CHECK_MIN_INTERVAL_SECS);
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
pub fn get_battery_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).battery_canister)
}

#[query]
#[candid_method(query)]
pub fn get_tools_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).tools_canister)
}

#[query]
#[candid_method(query)]
pub fn get_vector_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).vector_canister)
}

#[query]
#[candid_method(query)]
pub fn get_beamfi_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).beamfi_canister)
}

#[query]
#[candid_method(query)]
pub fn get_num_thoughts_processed() -> u64 {
    STATE.with(|state| (*state.borrow()).num_thoughts_processed)
}

#[query]
#[candid_method(query)]
pub fn get_max_num_thoughts_allowed() -> u64 {
    STATE.with(|state| (*state.borrow()).max_num_thoughts_allowed)
}

#[query]
#[candid_method(query)]
pub fn is_exceed_max_num_thoughts_allowed() -> bool {
    let num_thoughts_processed = get_num_thoughts_processed();
    let max_num_thoughts_allowed = get_max_num_thoughts_allowed();

    return num_thoughts_processed > max_num_thoughts_allowed;
}

#[query]
#[candid_method(query)]
pub fn is_paused() -> bool {
    return STATE.with(|state| (*state.borrow()).is_pause_chain_of_thoughts.unwrap());
}

#[update(guard = "assert_owner")]
#[candid_method(update)]
pub fn update_owner(new_owner: Principal) {
    STATE.with(|state| {
        state.borrow_mut().owner = Some(new_owner);
    });
}

#[update(guard = "assert_owner")]
#[candid_method(update)]
pub fn toggle_pause_cof() {
    let cur_pause = STATE
        .with(|state| (*state.borrow()).is_pause_chain_of_thoughts)
        .unwrap();
    STATE.with(|state| {
        state.borrow_mut().is_pause_chain_of_thoughts = Some(!cur_pause);
    });
}

#[query]
#[candid_method(query)]
pub fn get_version() -> u16 {
    3
}

/// Returns the amount of cycles used since the beginning of the execution.
#[query]
#[candid_method(query)]
fn cycles_used() -> u64 {
    CYCLES_USED.load(Ordering::Relaxed)
}

#[update]
#[candid_method(update)]
// Idempotent function to increment max_num_thoughts_allowed for the payment_transaction_id, only apply one
pub fn inc_max_num_thoughts_limit(
    billing_key: String,
    payment_transcation_id: String,
    add_limit: u32,
) {
    // get current billing_key
    let cur_billing_key = STATE
        .with(|state| (*state.borrow()).billing_key.clone())
        .unwrap();

    // verify billing_key == cur_billing_key
    if billing_key != cur_billing_key {
        ic_cdk::trap("Invalid billing key.");
    }

    // get payment transaction
    let payment_transaction_vec: Vec<PaymentTransaction> = get_paymenttransction();
    let payment_transaction: Option<&PaymentTransaction> = payment_transaction_vec
        .iter()
        .find(|&x| x.transaction_id == payment_transcation_id);

    // Increment max_num_thoughts_allowed if transaction does not exist
    if payment_transaction.is_none() {
        ic_cdk::println!("inc_max_num_thoughts_limit: adding new limt: {}", add_limit);

        let cur_max_num_thoughts_allowed = get_max_num_thoughts_allowed();
        STATE.with(|state| {
            state.borrow_mut().max_num_thoughts_allowed =
                cur_max_num_thoughts_allowed + add_limit as u64;
        });

        // insert payment transaction
        let now: Timestamp = time();
        let new_payment_transaction = PaymentTransaction {
            transaction_id: payment_transcation_id.clone(),
            created_at: now,
        };

        STATE.with(|s| {
            s.borrow_mut()
                .stable_paymenttransaction_data
                .push(&new_payment_transaction)
                .expect("call to inc_max_num_thoughts_limit failed")
        });
    } else {
        ic_cdk::println!("inc_max_num_thoughts_limit: the payment transaction already exists");
    }
}

#[ic_cdk::query]
fn http_request(request: HttpRequest) -> HttpResponse {
    let path = get_path(request.url.as_str()).unwrap_or("/");
    ic_cdk::println!("http_request: {}", path);

    match path {
        "/inc_max_num_thoughts_limit" => {
            return HttpResponse {
                status_code: 200,
                headers: Vec::new(),
                body: Vec::new(),
                upgrade: Some(true),
            };
        }
        _ => HttpResponse {
            status_code: 404,
            headers: Vec::new(),
            body: path.as_bytes().to_vec(),
            upgrade: Some(false),
        },
    }
}

#[ic_cdk::update]
fn http_request_update(request: HttpRequest) -> HttpResponse {
    let path = get_path(request.url.as_str()).unwrap_or("/");
    ic_cdk::println!("http_request_update: {}", path);

    match path {
        "/inc_max_num_thoughts_limit" => {
            let body = request.body.clone();

            //convert body to JSON string
            let body_str = String::from_utf8(body).unwrap();

            // parse body_str to JSON object
            let body_json = serde_json::from_str::<serde_json::Value>(&body_str);
            if body_json.is_err() {
                return HttpResponse {
                    status_code: 400,
                    headers: Vec::new(),
                    body: "Invalid JSON body".as_bytes().to_vec(),
                    upgrade: Some(false),
                };
            }
            // get billing_key from body_json
            let body_json_unwrap = body_json.unwrap();
            let billing_key = body_json_unwrap["billing_key"].as_str();
            if billing_key.is_none() {
                return HttpResponse {
                    status_code: 400,
                    headers: Vec::new(),
                    body: "Invalid billing_key".as_bytes().to_vec(),
                    upgrade: Some(false),
                };
            }

            // get payment_transcation_id from body_json
            let payment_transcation_id = body_json_unwrap["payment_transcation_id"].as_str();
            if payment_transcation_id.is_none() {
                return HttpResponse {
                    status_code: 400,
                    headers: Vec::new(),
                    body: "Invalid payment_transcation_id".as_bytes().to_vec(),
                    upgrade: Some(false),
                };
            }

            // get add_limit from body_json
            let add_limit = body_json_unwrap["add_limit"].as_i64();
            if add_limit.is_none() {
                return HttpResponse {
                    status_code: 400,
                    headers: Vec::new(),
                    body: "Invalid add_limit".as_bytes().to_vec(),
                    upgrade: Some(false),
                };
            }

            // convert add_limit to u32
            let add_limit = add_limit.unwrap_or(0) as u32;

            inc_max_num_thoughts_limit(
                billing_key.unwrap().to_string(),
                payment_transcation_id.unwrap().to_string(),
                add_limit,
            );

            return HttpResponse {
                status_code: 200,
                headers: Vec::new(),
                body: Vec::new(),
                upgrade: Some(false),
            };
        }
        _ => HttpResponse {
            status_code: 404,
            headers: Vec::new(),
            body: path.as_bytes().to_vec(),
            upgrade: Some(false),
        },
    }
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

    // Start the periodic tasks
    start_new_goals_check_timer(NEW_GOALS_CHECK_MIN_INTERVAL_SECS);
    start_cycles_check_timer(CYCLES_BALANCE_CHECK_MIN_INTERVAL_SECS);
}

// ---------------------- Periodic Task Timer ----------------------------------------------
#[update]
fn start_new_goals_check_timer(secs: u64) {
    let secs = Duration::from_secs(secs);
    ic_cdk::println!(
        "Controller canister: checking for new scheduled goal with {secs:?} interval..."
    );

    let timer_id = ic_cdk_timers::set_timer_interval(secs, || ic_cdk::spawn(process_new_goals()));

    // Add the timer ID to the global vector.
    TIMER_IDS.with(|timer_ids| timer_ids.borrow_mut().push(timer_id));
}

#[update]
fn start_cycles_check_timer(secs: u64) {
    let secs = Duration::from_secs(secs);
    ic_cdk::println!(
        "Controller canister: checking its cycles balance and request topup with {secs:?} interval..."
    );

    let timer_id =
        ic_cdk_timers::set_timer_interval(secs, || ic_cdk::spawn(check_cycles_and_topup()));

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
