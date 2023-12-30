# Validate required env vars
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

CONTROLLER_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)

# Deploy tools canister
echo Deploying tools canister with owner=$CONTROLLER_PRINCIPAL on $IC_NETWORK, GOOGLE_API_KEY=$GOOGLE_API_KEY, GOOGLE_SEARCH_ENGINE_ID=$GOOGLE_SEARCH_ENGINE_ID
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$CONTROLLER_PRINCIPAL\", \"$GOOGLE_API_KEY\", \"$GOOGLE_SEARCH_ENGINE_ID\")"

echo Tools Owner:
dfx canister --network $IC_NETWORK call arcmindai_tools get_owner
