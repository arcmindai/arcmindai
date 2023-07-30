dfx identity use default
NewPrincipal=$(dfx identity get-principal)
echo NewPrincipal=$NewPrincipal

dfx identity use icprod
dfx canister call arcmindai_controller update_owner "(principal \"$NewPrincipal\")"
dfx canister call arcmindai_controller get_owner