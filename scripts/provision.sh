# Validate required env vars
if [[ -z "${OPENAI_API_KEY}" ]]; then
  echo "OPENAI_API_KEY is unset."
  exit 1
fi

if [[ -z "${VECTOR_PRINCIPAL}" ]]; then
  echo "VECTOR_PRINCIPAL is unset."
  exit 1
fi

if [[ -z "${GOOGLE_API_KEY}" ]]; then
  echo "GOOGLE_API_KEY is unset."
  exit 1
fi

if [[ -z "${GOOGLE_SEARCH_ENGINE_ID}" ]]; then
  echo "GOOGLE_SEARCH_ENGINE_ID is unset."
  exit 1
fi


# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}
echo Provisioning on $IC_NETWORK


GPT_MODEL=gpt-4
BROWSE_WEBSITE_GPT_MODEL=gpt-3.5-turbo-1106

# To run this script, set following Env Vars in your terminal or uncomment below to set it
# OPENAI_API_KEY="UPDATE_ME"
echo OPENAI_API_KEY = $OPENAI_API_KEY

OWENR_PRINCIPAL=$(dfx identity --network $IC_NETWORK get-principal)

# Deploy brain canister 
echo Deploying brain canister with owner $OWENR_PRINCIPAL, GPT model $GPT_MODEL and openai_api_key $OPENAI_API_KEY
dfx deploy --network $IC_NETWORK arcmindai_brain --argument "(opt principal \"$OWENR_PRINCIPAL\", \"$OPENAI_API_KEY\", \"$GPT_MODEL\")"

BRAIN_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_brain)

# Deploy tools canister
echo Deploying tools canister with owner $OWENR_PRINCIPAL on $IC_NETWORK with GOOGLE_API_KEY=$GOOGLE_API_KEY, GOOGLE_SEARCH_ENGINE_ID=$GOOGLE_SEARCH_ENGINE_ID
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$OWENR_PRINCIPAL\", \"$GOOGLE_API_KEY\", \"$GOOGLE_SEARCH_ENGINE_ID\")"

TOOLS_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_tools)

# Deploy controller canister
echo Deploying controller canister with owner $OWENR_PRINCIPAL, brain $BRAIN_PRINCIPAL, tools $TOOLS_PRINCIPAL, vector $VECTOR_PRINCIPAL and browse_website_gpt_model $BROWSE_WEBSITE_GPT_MODEL on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OWENR_PRINCIPAL\", opt principal \"$BRAIN_PRINCIPAL\", opt principal \"$TOOLS_PRINCIPAL\", opt principal \"$VECTOR_PRINCIPAL\", opt \"$BROWSE_WEBSITE_GPT_MODEL\")"
echo Controller Owner:
dfx canister --network $IC_NETWORK call arcmindai_controller get_owner

# Update brain owner to controller
CONTROLLER_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
echo Updating brain owner to controller $CONTROLLER_PRINCIPAL
dfx canister --network $IC_NETWORK call arcmindai_brain update_owner "(principal \"$CONTROLLER_PRINCIPAL\")"

echo Brain Owner:
dfx canister --network $IC_NETWORK call arcmindai_brain get_owner

# Update tools owner to controller
echo Updating tools owner to controller $CONTROLLER_PRINCIPAL
dfx canister --network $IC_NETWORK call arcmindai_tools update_owner "(principal \"$CONTROLLER_PRINCIPAL\")"

echo Tools Owner:
dfx canister --network $IC_NETWORK call arcmindai_tools get_owner
