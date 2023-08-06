use std::{cell::RefCell, ops::Deref};

use ic_cdk::api::management_canister::http_request::{
    http_request, CanisterHttpRequestArgument, HttpHeader, HttpMethod,
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

use crate::guards::assert_owner;
mod guards;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub openai_api_key: String,
}

// Mutable global state
thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

// ---------------------- ArcMind AI Agent ----------------------
const OPENAI_HOST: &str = "api.openai.com";
const OPENAI_URL: &str = "https://api.openai.com/v1/chat/completions";

// entry function for user to ask questions
#[update(guard = "assert_owner")]
#[candid_method(update)]
async fn ask(question: String) -> String {
    let openai_api_key = STATE.with(|state| (*state.borrow()).openai_api_key.clone());
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

    let request_body = json!({
        "model": "gpt-3.5-turbo",
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

    let request = CanisterHttpRequestArgument {
        url: OPENAI_URL.to_string(),
        max_response_bytes: None,
        method: HttpMethod::POST,
        headers: request_headers,
        body: request_body,
        transform: None,
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

// ---------------------- Supporting Functions ----------------------

// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(owner: Option<Principal>, openai_api_key: String) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            openai_api_key: openai_api_key,
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
        *state.borrow_mut() = State {
            owner: Some(new_owner),
            openai_api_key: open_api_key,
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
