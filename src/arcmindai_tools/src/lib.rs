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

use html2text;

use crate::guards::assert_owner;
mod guards;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
}

// Mutable global state
thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

// entry function for user to web scrap a website
// TODO - add owner check back when full ArcMind AI is ready
#[update]
#[candid_method(update)]
async fn browse_website(url: String) -> String {
    let request_headers = vec![
        HttpHeader {
            name: "User-Agent".to_string(),
            value: "ArcMind AI Agent".to_string(),
        },
        HttpHeader {
            name: "Content-Type".to_string(),
            value: "application/json".to_string(),
        },
    ];

    let request_body = json!({});

    let json_utf8: Vec<u8> = request_body.to_string().into_bytes();
    let request_body: Option<Vec<u8>> = Some(json_utf8);
    ic_cdk::api::print(format!(
        "\n ------------- Web Scrap URL -------------\n{:?}",
        url
    ));

    let request = CanisterHttpRequestArgument {
        url: url.clone(),
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

#[query]
fn transform(args: TransformArgs) -> HttpResponse {
    let mut res = HttpResponse {
        status: args.response.status.clone(),
        ..Default::default()
    };

    if res.status == 200 {
        let mut res_str = String::from_utf8(args.response.body.clone())
            .expect("Transformed response is not UTF-8 encoded.");
        res_str = html2text::from_read(res_str.as_bytes(), 80);
        ic_cdk::api::print(format!(
            "\n ------------- HTML2Text -------------\n{:?}",
            res_str
        ));

        res.body = res_str.as_bytes().to_vec();
    } else {
        ic_cdk::api::print(format!("Received an error from jsonropc: err = {:?}", args));
    }

    res
}

// ---------------------- Supporting Functions ----------------------

// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(owner: Option<Principal>) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
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
        *state.borrow_mut() = State {
            owner: Some(new_owner),
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
        write(dir.join("arcmindai_tools.did"), __export_service()).expect("Write failed.");
    }
}
