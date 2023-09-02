# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=local

OwnerPrincipal=$(dfx identity --network $IC_NETWORK get-principal)
BrainPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_brain)

# Deploy controller canister
echo Deploying controller canister with owner $OwnerPrincipal and brain $BrainPrincipal
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OwnerPrincipal\", opt principal \"$BrainPrincipal\")"
