use std::{cell::RefCell, ops::Deref};

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod, HttpResponse, TransformArgs,
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

use ic_cdk::api::time;

mod guards;
use guards::assert_owner;

type Timestamp = u64;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub openai_api_key: String,
    pub gpt_model: String,
}

// Mutable global state
thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

// ---------------------- ArcMind AI Agent ----------------------
const OPENAI_HOST: &str = "openai-4gbndkvjta-uc.a.run.app";
const OPENAI_URL: &str = "https://openai-4gbndkvjta-uc.a.run.app";

// entry function for user to ask questions
#[update(guard = "assert_owner")]
#[candid_method(update)]
async fn ask(question: String) -> String {
    let openai_api_key = STATE.with(|state| (*state.borrow()).openai_api_key.clone());
    let gpt_model = STATE.with(|state| (*state.borrow()).gpt_model.clone());

    let request_headers = vec![
        HttpHeader {
            name: "Host".to_string(),
            value: format!("{OPENAI_HOST}:443"),
        },
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "ArcMind AI Agent".to_string(),
        },
        HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        },
        HttpHeader {
            name: "Authorization".to_string(),
            value: format!("Bearer {openai_api_key}").to_string(),
        },
    ];

    // lower temperature = more predictable and deterministic response = less creative
    // so that IC replicas can reach consensus on the response
    let request_body = json!({
        "model": gpt_model,
        "messages": [
            {
                "role": "user",
                "content": question
            }
        ],
        "temperature": 0.7
    });

    let json_utf8: Vec<u8> = request_body.to_string().into_bytes();
    let request_body: Option<Vec<u8>> = Some(json_utf8);

    // extract the first 5 characters from the request_id
    let canister_id = api::id().to_text();
    let init_canister_id = canister_id.chars().take(5).collect::<String>();
    let now: Timestamp = time();
    let request_id = format!("{}-{}", init_canister_id, now);

    // add requestId to OPENAI_URL
    let final_url = OPENAI_URL.to_string() + "?requestId=" + &request_id;
    let request = CanisterHttpRequestArgument {
        url: final_url.to_string(),
        max_response_bytes: None,
        method: HttpMethod::POST,
        headers: request_headers,
        body: request_body,
        transform: Some(TransformContext::new(transform, vec![])),
    };

    match http_request(request).await {
        Ok((response,)) => {
            let result = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            result
        }
        Err((r, m)) => {
            let message = format!("The ask resulted into error. RejectionCode: {r:?}, Error: {m}");
            message
        }
    }
}

#[derive(serde::Serialize, Deserialize)]
struct OpenAIResult {
    id: String,
    object: String,
    created: u32,
    model: String,
    choices: Vec<OpenAIResultChoices>,
    usage: OpenAIResultUsage,
}

#[derive(serde::Serialize, Deserialize)]
struct OpenAIResultChoices {
    index: u8,
    message: OpenAIResultMessage,
    finish_reason: String,
}

#[derive(serde::Serialize, Deserialize)]
struct OpenAIResultMessage {
    role: String,
    content: String,
}

#[derive(serde::Serialize, Deserialize)]
struct OpenAIResultUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[query]
fn transform(args: TransformArgs) -> HttpResponse {
    let mut res = HttpResponse {
        status: args.response.status.clone(),
        ..Default::default()
    };

    if res.status == 200 {
        let res_str = String::from_utf8(args.response.body.clone())
            .expect("Transformed response is not UTF-8 encoded.");
        println!("res_str = {:?}", res_str);
        let json_str = res_str.replace("\n", "");

        ic_cdk::api::print(format!("JSON str = {:?}", json_str));

        let r: OpenAIResult = serde_json::from_str(json_str.as_str()).unwrap();
        let content = &r.choices[0].message.content;

        res.body = content.as_bytes().to_vec();
    } else {
        ic_cdk::api::print(format!("Received an error from jsonropc: err = {:?}", args));
    }

    res
}

// ---------------------- Supporting Functions ----------------------

// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(owner: Option<Principal>, openai_api_key: String, gpt_model: String) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            openai_api_key: openai_api_key,
            gpt_model: gpt_model,
        };
    });
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
        *state.borrow_mut() = State {
            owner: Some(new_owner),
            openai_api_key: open_api_key,
            gpt_model: gpt_model,
        };
    });
}

#[pre_upgrade]
fn pre_upgrade() {
    STATE.with(|cell| {
        ciborium::ser::into_writer(cell.borrow().deref(), StableWriter::default())
            .expect("failed to encode state")
    })
}

#[post_upgrade]
fn post_upgrade() {
    STATE.with(|cell| {
        *cell.borrow_mut() =
            ciborium::de::from_reader(StableReader::default()).expect("failed to decode state");
    })
}

// ---------------------- Candid declarations did file generator ----------------------
#[cfg(test)]
mod tests {
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
