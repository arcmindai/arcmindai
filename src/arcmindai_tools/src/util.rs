use ic_cdk::api::{self};

use ic_cdk::api::time;
type Timestamp = u64;

pub fn generate_request_id() -> String {
    // extract the first 5 characters from the request_id
    let canister_id = api::id().to_text();
    let init_canister_id = canister_id.chars().take(5).collect::<String>();
    let now: Timestamp = time();
    let request_id = format!("{}-{}", init_canister_id, now);

    return request_id;
}
