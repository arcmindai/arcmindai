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
use urlencoding::encode;

mod guards;
use guards::assert_owner;

use tinytemplate::TinyTemplate;

mod config;
use config::GOOGLE_SEARCH_URL;

mod util;
use util::generate_request_id;

const BROWSE_WEBSITE_PROXY_URL: &str = "https://browsewebsite-4gbndkvjta-uc.a.run.app";
const MAX_NUM_GOOGLE_SEARCH_RESULTS: i32 = 3;

#[derive(Default, CandidType, Serialize, Deserialize)]
pub struct State {
    pub owner: Option<Principal>,
    pub google_api_key: String,
    pub search_engine_id: String,
}

#[derive(Serialize)]
struct GoogleSearchContext {
    google_api_key: String,
    search_engine_id: String,
    query: String,
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
    let request_headers = vec![HttpHeader {
        name: "User-Agent".to_string(),
        value: "ArcMind AI Agent".to_string(),
    }];

    let url_encoded_weburl = encode(url.as_str());
    let request_id = generate_request_id();

    // add requestId to OPENAI_URL
    let final_url = BROWSE_WEBSITE_PROXY_URL.to_string()
        + "?requestId="
        + &request_id
        + "&webURL="
        + &url_encoded_weburl;

    ic_cdk::api::print(format!(
        "\n ------------- Browse Website URL -------------\n{:?}",
        final_url
    ));

    let request = CanisterHttpRequestArgument {
        url: final_url.clone(),
        max_response_bytes: None,
        method: HttpMethod::GET,
        headers: request_headers,
        body: None,
        transform: Some(TransformContext::new(transform, vec![])),
    };

    match http_request(request).await {
        Ok((response,)) => {
            let result = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            result
        }
        Err((r, m)) => {
            let message =
                format!("The browse_website resulted into error. RejectionCode: {r:?}, Error: {m}");
            message
        }
    }
}

// entry function for user to perform google search on a query
// TODO - add owner check back when full ArcMind AI is ready
#[update]
#[candid_method(update)]
async fn google(query: String) -> String {
    let request_headers = vec![HttpHeader {
        name: "User-Agent".to_string(),
        value: "ArcMind AI Agent".to_string(),
    }];

    ic_cdk::api::print(format!(
        "\n ------------- Google Search -------------\n{:?}",
        query
    ));

    let mut tt = TinyTemplate::new();
    tt.add_template("google_search", GOOGLE_SEARCH_URL).unwrap();

    let google_api_key = STATE.with(|state| (*state.borrow()).google_api_key.clone());
    let search_engine_id = STATE.with(|state| (*state.borrow()).search_engine_id.clone());
    let url_encoded_query = encode(query.as_str());

    let context = GoogleSearchContext {
        google_api_key: google_api_key.to_string(),
        search_engine_id: search_engine_id.to_string(),
        query: url_encoded_query.to_string(),
    };

    let google_url = tt.render("google_search", &context).unwrap();
    let request_id = generate_request_id();
    let final_url = google_url.to_string() + "&requestId=" + &request_id;

    let request = CanisterHttpRequestArgument {
        url: final_url,
        max_response_bytes: None,
        method: HttpMethod::GET,
        headers: request_headers,
        body: None,
        transform: Some(TransformContext::new(transform, vec![])),
    };

    match http_request(request).await {
        Ok((response,)) => {
            let result = String::from_utf8(response.body)
                .expect("Transformed response is not UTF-8 encoded.");
            result
        }
        Err((r, m)) => {
            let message =
                format!("The google resulted into error. RejectionCode: {r:?}, Error: {m}");
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
        // check if res_str is JSON
        let res_json = serde_json::from_str::<serde_json::Value>(&res_str);
        if res_json.is_err() {
            // If not JSON, convert HTML to text for browse_website response
            res_str = html2text::from_read(res_str.as_bytes(), 80);
            res.body = res_str.as_bytes().to_vec();
            return res;
        }

        // Assume this is Google Search result, convert to JSON object
        // extract items[].title, link, snippet into JSON array object
        let mut res_json_mut = res_json.unwrap();
        let res_items = res_json_mut["items"].as_array_mut().unwrap();
        let mut res_items_arr = Vec::new();
        let mut num_items = 0;

        for item in res_items.iter_mut() {
            if num_items >= MAX_NUM_GOOGLE_SEARCH_RESULTS {
                break;
            }

            let item_json = json!({
                "title": item["title"],
                "link": item["link"],
                "snippet": item["snippet"]
            });
            res_items_arr.push(item_json);

            num_items += 1;
        }

        // convert JSON array object to string
        res_str = serde_json::to_string(&res_items_arr).unwrap();
        res.body = res_str.as_bytes().to_vec();
        return res;
    }

    // Error status handling
    let res_str = String::from_utf8(args.response.body.clone())
        .expect("Transformed response is not UTF-8 encoded.");
    ic_cdk::api::print(format!(
        "\n ------------- Response -------------\n{:?}",
        res_str
    ));

    ic_cdk::api::print(format!("\n\nReceived an error from transform: {:?}", args));
    res
}

// ---------------------- Supporting Functions ----------------------

// Controller canister must be created with principal
#[init]
#[candid_method(init)]
fn init(owner: Option<Principal>, google_api_key: String, search_engine_id: String) {
    let my_owner: Principal = owner.unwrap_or_else(|| api::caller());
    STATE.with(|state| {
        *state.borrow_mut() = State {
            owner: Some(my_owner),
            google_api_key: google_api_key,
            search_engine_id: search_engine_id,
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
            google_api_key: state.borrow().google_api_key.clone(),
            search_engine_id: state.borrow().search_engine_id.clone(),
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
