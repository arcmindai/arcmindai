use ic_cdk::api::management_canister::http_request::HttpHeader;
use ic_cdk::api::time;
use ic_cdk::api::{self};

use crate::datatype::Timestamp;

pub const OPENAI_HOST: &str = "openai-4gbndkvjta-uc.a.run.app";
pub const OPENAI_EMBEDDINGS_HOST: &str = "openaiembeddings-4gbndkvjta-uc.a.run.app";

pub const OPENAI_EMBEDDINGS_MODEL: &str = "text-embedding-ada-002";

pub fn create_header(openai_api_key: String, host: String, request_id: String) -> Vec<HttpHeader> {
    let request_headers = vec![
        HttpHeader {
            name: "authority".to_string(),
            value: host,
        },
        HttpHeader {
            name: "scheme".to_string(),
            value: "https".to_string(),
        },
        HttpHeader {
            name: "user-agent".to_string(),
            value: "ArcMind AI Agent".to_string(),
        },
        HttpHeader {
            name: "content-type".to_string(),
            value: "application/json".to_string(),
        },
        HttpHeader {
            name: "idempotency-key".to_string(),
            value: request_id,
        },
        HttpHeader {
            name: "accept".to_string(),
            value: "application/json".to_string(),
        },
        HttpHeader {
            name: "authorization".to_string(),
            value: format!("Bearer {openai_api_key}").to_string(),
        },
    ];

    return request_headers;
}

pub fn generate_request_id(opt_request_id: Option<String>) -> String {
    let request_id = match opt_request_id {
        Some(id) => id,
        None => {
            // extract the first 5 characters from the canister id
            let canister_id = api::id().to_text();
            let init_canister_id = canister_id.chars().take(5).collect::<String>();
            let now: Timestamp = time();

            format!("{}-{}", init_canister_id, now)
        }
    };

    return request_id;
}
