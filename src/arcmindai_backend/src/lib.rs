#[ic_cdk::query]
fn greet(name: String) -> String {
    format!(
        "Hello there, {}! This is an example greeting returned from a Rust backend canister!",
        name
    )
}
