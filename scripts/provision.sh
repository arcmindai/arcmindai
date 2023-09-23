# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=local
echo Provisioning on $IC_NETWORK

GPT_MODEL=gpt-4

# To run this script, set following Env Vars in your terminal or uncomment below to set it
# OPENAI_API_KEY="UPDATE_ME"
echo OPENAI_API_KEY = $OPENAI_API_KEY

OWENR_PRINCIPAL=$(dfx identity --network $IC_NETWORK get-principal)

# Deploy brain canister 
echo Deploying brain canister with owner $OWENR_PRINCIPAL, GPT model $GPT_MODEL and openai_api_key $OPENAI_API_KEY
dfx deploy --network $IC_NETWORK arcmindai_brain --argument "(opt principal \"$OWENR_PRINCIPAL\", \"$OPENAI_API_KEY\", \"$GPT_MODEL\")"

BRAIN_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_brain)

# Deploy controller canister
echo Deploying controller canister with owner $OWENR_PRINCIPAL and brain $BRAIN_PRINCIPAL
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OWENR_PRINCIPAL\", opt principal \"$BRAIN_PRINCIPAL\")"
echo Controller Owner:
dfx canister --network $IC_NETWORK call arcmindai_controller get_owner

# Update brain owner to controller
CONTROLLER_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
echo Updating brain owner to controller $CONTROLLER_PRINCIPAL
dfx canister --network $IC_NETWORK call arcmindai_brain update_owner "(principal \"$CONTROLLER_PRINCIPAL\")"

echo Brain Owner:
dfx canister --network $IC_NETWORK call arcmindai_brain get_owner

# Deploy tools canister
echo Deploying tools canister with owner $OWENR_PRINCIPAL on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$OWENR_PRINCIPAL\", \"$GOOGLE_API_KEY\", \"$GOOGLE_SEARCH_ENGINE_ID\")"