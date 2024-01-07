use async_trait::async_trait;
use candid::Principal;

#[async_trait]
pub trait AMPluginAction {
    // Associated function signature; `Self` refers to the implementor type.
    fn new() -> Self;
    async fn invoke(
        &self,
        controller_canister: Principal,
        beamfi_canister: Principal,
        args: Vec<String>,
    ) -> String;
    fn get_name(&self) -> &'static str;
    fn get_command(&self) -> &'static str;
    fn get_args(&self) -> Vec<&'static str>;
}
