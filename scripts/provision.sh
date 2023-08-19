# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=local
echo Provisioning on $IC_NETWORK

GPT_MODEL=gpt-4

# To run this script, set following Env Vars in your terminal or uncomment below to set it
# OPENAI_API_KEY="UPDATE_ME"
echo OPENAI_API_KEY = $OPENAI_API_KEY

OwnerPrincipal=$(dfx identity --network $IC_NETWORK get-principal)

# Deploy brain canister 
echo Deploying brain canister with owner $OwnerPrincipal, GPT model $GPT_MODEL and openai_api_key $OPENAI_API_KEY
dfx deploy --network $IC_NETWORK arcmindai_brain --argument "(opt principal \"$OwnerPrincipal\", \"$OPENAI_API_KEY\", \"$GPT_MODEL\")"

BrainPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_brain)

# Deploy controller canister
echo Deploying controller canister with owner $OwnerPrincipal and brain $BrainPrincipal
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OwnerPrincipal\", opt principal \"$BrainPrincipal\")"
echo Controller Owner:
dfx canister --network $IC_NETWORK call arcmindai_controller get_owner

# Update brain owner to controller
ControllerPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
echo Updating brain owner to controller $ControllerPrincipal
dfx canister --network $IC_NETWORK call arcmindai_brain update_owner "(principal \"$ControllerPrincipal\")"

echo Brain Owner:
dfx canister --network $IC_NETWORK call arcmindai_brain get_owner