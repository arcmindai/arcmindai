use async_trait::async_trait;
use candid::{Int, Principal};
use ic_cdk::api::time;
use ic_principal::Principal as ICPrincipal;

use candid::{CandidType, Deserialize};

use ic_ledger_types::{
    transfer, AccountIdentifier, BlockIndex, Memo, Tokens, TransferArgs, DEFAULT_FEE,
    DEFAULT_SUBACCOUNT, MAINNET_LEDGER_CANISTER_ID,
};

use crate::{datatype::Timestamp, plugin_types::AMPluginAction};

// 24 hours in nano seconds
const DUE_DATE_DURATION: u64 = 24 * 60 * 60 * 1000 * 1000 * 1000;

pub struct BeamFiPlugin {
    pub name: &'static str,
    pub command: &'static str,
    pub args: Vec<&'static str>,
}

#[derive(CandidType, Deserialize)]
enum TokenType {
    #[serde(rename = "icp")]
    ICP,
}

#[derive(CandidType, Deserialize, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
enum CandidResult<T, E> {
    #[serde(rename = "ok")]
    Ok(T),
    #[serde(rename = "err")]
    Err(E),
}

#[derive(CandidType, Deserialize, Debug)]
enum CreateBeamEscrowResponseErrorCode {
    #[serde(rename = "escrow_payment_not_found")]
    EscrowPaymentNotFound(String),
    #[serde(rename = "escrow_contract_verification_failed")]
    EscrowContractVerificationFailed(String),
    #[serde(rename = "escrow_token_owned_not_matched")]
    EscrowTokenOwnedNotMatched(String),
    #[serde(rename = "escrow_contract_not_found")]
    EscrowContractNotFound(String),
    #[serde(rename = "escrow_beam_failed")]
    EscrowBeamFailed(String),
}

impl BeamFiPlugin {
    async fn stream_payment(
        &self,
        controller_canister: Principal,
        beamfi_canister: Principal,
        args: Vec<String>,
    ) -> String {
        let amount: u64 = args[0].parse().unwrap();
        let amount_e8s: u64 = amount * 100_000_000;

        let token_type: String = args[1].parse().unwrap();
        let token_type_enum: TokenType = match token_type.as_str() {
            "ICP" => TokenType::ICP,
            _ => TokenType::ICP,
        };

        let recipient_principal_id: String = args[2].parse().unwrap();
        let recipient_principal = Principal::from_text(recipient_principal_id.clone()).unwrap();

        // transfer ICP from controller to BeamEscrow canister, assuming controller has enough ICP
        let block_index: u64 = self.transfer_icp(amount, beamfi_canister).await;

        //  due_date in UTC epoch nanoseconds from now + 24 hrs
        let due_date: Timestamp = time() + DUE_DATE_DURATION;
        let due_date_int: Int = due_date.into();

        let (result,): (CandidResult<u32, CreateBeamEscrowResponseErrorCode>,) =
            ic_cdk::api::call::call(
                beamfi_canister,
                "createBeamEscrow",
                (
                    amount_e8s,
                    token_type_enum,
                    block_index,
                    due_date_int,
                    controller_canister,
                    recipient_principal,
                ),
            )
            .await
            .expect("call to createBeamEscrow failed");

        // if result is error, panic, else return the escrow_id
        let escrow_id: u32 = match result {
            CandidResult::Ok(escrow_id) => escrow_id,
            CandidResult::Err(error_code) => {
                panic!("createBeamEscrow failed with error code: {:?}", error_code)
            }
        };

        return escrow_id.to_string();
    }

    async fn transfer_icp(&self, amount: u64, recipient_principal: Principal) -> BlockIndex {
        // convert amount to E8S format with a base of 8 zeros
        let amount_in_e8s = amount * 100_000_000;
        let to_principal: ICPrincipal =
            ICPrincipal::from_text(recipient_principal.to_text()).unwrap();

        let block_index = transfer(
            MAINNET_LEDGER_CANISTER_ID,
            TransferArgs {
                memo: Memo(0),
                amount: Tokens::from_e8s(amount_in_e8s),
                fee: DEFAULT_FEE,
                from_subaccount: None,
                to: AccountIdentifier::new(&to_principal, &DEFAULT_SUBACCOUNT),
                created_at_time: None,
            },
        )
        .await
        .expect("call to ledger failed")
        .expect("transfer failed");

        return block_index;
    }
}

#[async_trait]
impl AMPluginAction for BeamFiPlugin {
    // `Self` is the implementor type: `BeamFiPlugin`.
    fn new() -> BeamFiPlugin {
        BeamFiPlugin {
            name: "BeamFi stream payment",
            command: "beamfi_stream_payment",
            args: ["amount", "token_type", "recipient_principal_id"].to_vec(),
        }
    }

    async fn invoke(
        &self,
        controller_canister: Principal,
        beamfi_canister: Principal,
        args: Vec<String>,
    ) -> String {
        return self
            .stream_payment(controller_canister, beamfi_canister, args)
            .await;
    }

    fn get_name(&self) -> &'static str {
        return self.name;
    }

    fn get_command(&self) -> &'static str {
        return self.command;
    }

    fn get_args(&self) -> Vec<&'static str> {
        return self.args.clone();
    }
}
