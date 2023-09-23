# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=local

OwnerPrincipal=$(dfx identity --network $IC_NETWORK get-principal)

# Deploy tools canister
echo Deploying tools canister with owner $OwnerPrincipal
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$OwnerPrincipal\")"

ToolsPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_tools)
