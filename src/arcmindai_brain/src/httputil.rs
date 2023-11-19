use ic_cdk::api::management_canister::http_request::HttpHeader;
use ic_cdk::api::time;
use ic_cdk::api::{self};

use crate::datatype::Timestamp;

pub const OPENAI_HOST: &str = "openai-4gbndkvjta-uc.a.run.app";
pub const OPENAI_EMBEDDINGS_HOST: &str = "openaiembeddings-4gbndkvjta-uc.a.run.app";

pub const OPENAI_EMBEDDINGS_MODEL: &str = "text-embedding-ada-002";

pub fn create_header(openai_api_key: String) -> Vec<HttpHeader> {
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

    return request_headers;
}

pub fn generate_request_id(opt_request_id: Option<String>) -> String {
    // extract the first 5 characters from the request_id
    let canister_id = api::id().to_text();
    let init_canister_id = canister_id.chars().take(5).collect::<String>();
    let now: Timestamp = time();
    let request_id = match opt_request_id {
        Some(id) => id,
        None => format!("{}-{}", init_canister_id, now),
    };

    return request_id;
}
