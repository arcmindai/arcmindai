use std::{cell::RefCell, ops::Deref, time::Duration};

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpMethod, HttpResponse, TransformArgs,
    TransformContext,
};

use candid::{candid_method, CandidType, Deserialize, Principal};
use ic_cdk::{
    api::{
        self,
        stable::{StableReader, StableWriter},
    },
    init, post_upgrade, pre_upgrade, query, update,
};
use serde::Serialize;
use serde_json::json;

mod datatype;
use datatype::{Embeddings, OpenAIEmbeddingResult, OpenAIResult};

mod guards;
use async_recursion::async_recursion;
use guards::assert_owner;

mod tokenutil;
use tokenutil::{truncate_question, MAX_16K_TOKENS};

mod httputil;
use httputil::{
    create_header, generate_request_id, OPENAI_EMBEDDINGS_HOST, OPENAI_EMBEDDINGS_MODEL,
    OPENAI_HOST,
};

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub openai_api_key: String,
    pub gpt_model: String,
    pub battery_api_key: Option<String>,
    pub battery_canister: Option<Principal>,
}

// Mutable global state
thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

// ---------------------- ArcMind AI Agent ----------------------
const MAX_DEFAULT_TOKENS: usize = 8000;
const MAX_NUM_RETIRES: i8 = 2;
const GPT_TEMPERATURE: f32 = 0.5;

// 3 days
const CYCLES_BALANCE_CHECK_MIN_INTERVAL_SECS: u64 = 60 * 60 * 24 * 3;
// Cycle usage threshold
const CYCLES_ONE_TC: u64 = 1_000_000_000_000;
const CYCLES_THRESHOLD: u64 = 3 * CYCLES_ONE_TC;
const CYCLES_TOPUP_AMT: u64 = 4 * CYCLES_ONE_TC;

const CYCLES_TOPUP_GROUP: &str = "arcmindai_brain";

// entry function for user to ask questions
#[update(guard = "assert_owner")]
#[candid_method(update)]
#[async_recursion]
async fn ask(
    question: String,
    custom_gpt_model: Option<String>,
    num_retries: i8,
    opt_request_id: Option<String>,
) -> String {
    // use custom gpt model if provided
    let gpt_model = match custom_gpt_model {
        Some(model) => model,
        None => STATE.with(|state| (*state.borrow()).gpt_model.clone()),
    };

    // Truncate question if reaching the max token limit of the model
    let max_token_limit = match gpt_model.as_str() {
        "gpt-3.5-turbo-16k" => MAX_16K_TOKENS,
        _ => MAX_DEFAULT_TOKENS,
    };

    // log gpt_model and max_token_limit
    ic_cdk::println!(
        "gpt_model: {}, max_token_limit: {}",
        gpt_model,
        max_token_limit
    );

    let safe_question = truncate_question(question.clone(), max_token_limit);

    // lower temperature = more predictable and deterministic response = less creative
    // so that IC replicas can reach consensus on the response
    let request_body = json!({
        "model": gpt_model,
        "messages": [
            {
                "role": "user",
                "content": safe_question
            }
        ],
        "temperature": GPT_TEMPERATURE
    });

    let json_utf8: Vec<u8> = request_body.to_string().into_bytes();
    let request_body_json: Option<Vec<u8>> = Some(json_utf8);
    let request_id = generate_request_id(opt_request_id);

    // add requestId to OPENAI_URL
    let openai_url = "https://".to_string() + OPENAI_HOST;
    let final_url = openai_url + "?requestId=" + &request_id;
    let openai_api_key = STATE.with(|state| (*state.borrow()).openai_api_key.clone());
    let headers = create_header(openai_api_key, OPENAI_HOST.to_string());

    let request = CanisterHttpRequestArgument {
        url: final_url.to_string(),
        max_response_bytes: None,
        method: HttpMethod::POST,
        headers: headers,
        body: request_body_json,
        transform: Some(TransformContext::new(
            transform_openai_chat_completion,
            vec![],
        )),
    };

    match http_request(request).await {
        Ok((response,)) => {
            let result = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            result
        }
        Err((r, m)) => {
            if num_retries < MAX_NUM_RETIRES {
                ic_cdk::println!("Retrying ask, num_retries: {}", num_retries);
                return ask(
                    question.clone(),
                    Some(gpt_model),
                    num_retries + 1,
                    Some(request_id),
                )
                .await;
            }

            let message = format!("The ask resulted into error. RejectionCode: {r:?}, Error: {m}");
            message
        }
    }
}

#[query]
fn transform_openai_chat_completion(args: TransformArgs) -> HttpResponse {
    let mut res = HttpResponse {
        status: args.response.status.clone(),
        ..Default::default()
    };

    if res.status == 200 {
        let res_str = String::from_utf8(args.response.body.clone())
            .expect("Transformed response is not UTF-8 encoded.");
        let json_str = res_str.replace("\n", "");

        let openai_result = serde_json::from_str(json_str.as_str());
        if openai_result.is_err() {
            res.body = format!("Invalid JSON str = {:?}", json_str)
                .as_bytes()
                .to_vec();
            return res;
        }

        let openai_body: OpenAIResult = openai_result.unwrap();
        let content = &openai_body.choices[0].message.content;
        res.body = content.as_bytes().to_vec();
        return res;
    }

    ic_cdk::api::print(format!("Received an error from jsonropc: err = {:?}", args));
    return res;
}

// entry function for user to ask questions
#[update(guard = "assert_owner")]
#[candid_method(update)]
#[async_recursion]
pub async fn generate_embeddings(
    content: String,
    num_retries: i8,
    opt_request_id: Option<String>,
) -> Result<Embeddings, String> {
    let request_body = json!({
        "input": content,
        "model": OPENAI_EMBEDDINGS_MODEL,
    });

    let json_utf8: Vec<u8> = request_body.to_string().into_bytes();
    let request_body_json: Option<Vec<u8>> = Some(json_utf8);
    let request_id = generate_request_id(opt_request_id);

    // add requestId to OPENAI_URL
    let openai_url = "https://".to_string() + OPENAI_EMBEDDINGS_HOST;
    let final_url = openai_url + "?requestId=" + &request_id;
    let openai_api_key = STATE.with(|state| (*state.borrow()).openai_api_key.clone());
    let headers = create_header(openai_api_key, OPENAI_EMBEDDINGS_HOST.to_string());

    let request = CanisterHttpRequestArgument {
        url: final_url.to_string(),
        max_response_bytes: None,
        method: HttpMethod::POST,
        headers: headers,
        body: request_body_json,
        transform: Some(TransformContext::new(transform_openai_embeddings, vec![])),
    };

    match http_request(request).await {
        Ok((response,)) => {
            let res_str = String::from_utf8(response.body.clone())
                .expect("Transformed response is not UTF-8 encoded.");

            let openai_result = serde_json::from_str(res_str.as_str());
            if openai_result.is_err() {
                let mesg = format!("Invalid JSON str = {:?}", res_str);
                return Err(mesg);
            }

            let openai_body: OpenAIEmbeddingResult = openai_result.unwrap();
            let embedding = &openai_body.data[0].embedding;
            return Ok(embedding.clone());
        }
        Err((r, m)) => {
            if num_retries < MAX_NUM_RETIRES {
                ic_cdk::println!("Retrying ask, num_retries: {}", num_retries);
                return generate_embeddings(content.clone(), num_retries + 1, Some(request_id))
                    .await;
            }

            let message = format!(
                "The generate_embeddings resulted into error. RejectionCode: {r:?}, Error: {m}"
            );
            return Err(message);
        }
    }
}

#[query]
fn transform_openai_embeddings(args: TransformArgs) -> HttpResponse {
    let mut res = HttpResponse {
        status: args.response.status.clone(),
        ..Default::default()
    };

    if res.status == 200 {
        let res_str = String::from_utf8(args.response.body.clone())
            .expect("Transformed response is not UTF-8 encoded.");
        let json_str = res_str.replace("\n", "");
        res.body = json_str.as_bytes().to_vec();
        return res;
    }

    ic_cdk::api::print(format!("Received an error from jsonropc: err = {:?}", args));
    return res;
}
// ---------------------- Supporting Functions ----------------------

// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(
    owner: Option<Principal>,
    openai_api_key: String,
    gpt_model: String,
    battery_api_key: Option<String>,
    battery_canister: Option<Principal>,
) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            openai_api_key: openai_api_key,
            gpt_model: gpt_model,
            battery_api_key: battery_api_key,
            battery_canister: battery_canister,
        };
    });

    start_cycles_check_timer(CYCLES_BALANCE_CHECK_MIN_INTERVAL_SECS);
}

#[query]
#[candid_method(query)]
pub fn get_owner() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).owner)
}

#[update(guard = "assert_owner")]
#[candid_method(update)]
pub fn update_owner(new_owner: Principal) {
    STATE.with(|state| {
        let open_api_key = state.borrow().openai_api_key.clone();
        let gpt_model = state.borrow().gpt_model.clone();
        let battery_api_key = state.borrow().battery_api_key.clone();
        let battery_canister = state.borrow().battery_canister.clone();

        *state.borrow_mut() = State {
            owner: Some(new_owner),
            openai_api_key: open_api_key,
            gpt_model: gpt_model,
            battery_api_key: battery_api_key,
            battery_canister: battery_canister,
        };
    });
}

#[update]
fn start_cycles_check_timer(secs: u64) {
    let secs = Duration::from_secs(secs);
    ic_cdk::println!(
        "Controller canister: checking its cycles balance and request topup with {secs:?} interval..."
    );

    ic_cdk_timers::set_timer_interval(secs, || ic_cdk::spawn(check_cycles_and_topup()));
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

#[query]
#[candid_method(query)]
pub fn get_battery_canister() -> Option<Principal> {
    STATE.with(|state| (*state.borrow()).battery_canister)
}

#[pre_upgrade]
fn pre_upgrade() {
    STATE.with(|cell| {
        ciborium::ser::into_writer(cell.borrow().deref(), StableWriter::default())
            .expect("failed to encode state")
    })
}

#[post_upgrade]
fn post_upgrade(
    _owner: Option<Principal>,
    _openai_api_key: String,
    _gpt_model: String,
    battery_api_key: Option<String>,
    battery_canister: Option<Principal>,
) {
    STATE.with(|cell| {
        *cell.borrow_mut() =
            ciborium::de::from_reader(StableReader::default()).expect("failed to decode state");
    });

    // Update newly added state in the latest version state using argument
    STATE.with(|s| {
        s.borrow_mut().battery_canister = battery_canister;
        s.borrow_mut().battery_api_key = battery_api_key.clone();
    });
}

// ---------------------- Candid declarations did file generator ----------------------
#[cfg(test)]
mod tests {
    use crate::datatype::Embeddings;
    use candid::{export_service, Principal};

    #[test]
    fn save_candid() {
        use std::env;
        use std::fs::write;
        use std::path::PathBuf;

        let dir = PathBuf::from(env::current_dir().unwrap());
        export_service!();
        write(dir.join("arcmindai_brain.did"), __export_service()).expect("Write failed.");
    }
}
