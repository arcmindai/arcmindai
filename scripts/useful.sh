MyPrincipal=$(dfx identity get-principal)
dfx deploy arcmindai_controller --argument "(principal \"$MyPrincipal\")"

dfx canister call arcmindai_controller greet '("Henry")'