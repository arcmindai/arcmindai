MyPrincipal=$(dfx identity get-principal)
echo $MyPrincipal
# dfx deploy arcmindai_controller
dfx deploy arcmindai_controller --argument "(opt principal \"$MyPrincipal\")"

# dfx canister call arcmindai_controller greet '("Henry")'