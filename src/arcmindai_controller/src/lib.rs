use std::{cell::RefCell, ops::Deref};

use candid::{candid_method, CandidType, Deserialize, Principal};
use ic_cdk::{
    api::{
        self,
        stable::{StableReader, StableWriter},
    },
    init, post_upgrade, pre_upgrade, query,
};
use serde::Serialize;

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

#[query(guard = "assert_owner")]
#[candid_method(query)]
fn greet(name: String) -> String {
    format!("Hello, {}!", name)
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
        write(dir.join("arcmindai_controller.did"), __export_service()).expect("Write failed.");
    }
}
