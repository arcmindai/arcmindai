#!/bin/bash

# Validate required env vars
if [[ -z "${GOOGLE_API_KEY}" ]]; then
  echo "GOOGLE_API_KEY is unset."
  exit 1
fi

if [[ -z "${GOOGLE_SEARCH_ENGINE_ID}" ]]; then
  echo "GOOGLE_SEARCH_ENGINE_ID is unset."
  exit 1
fi

if [[ -z "${BATTERY_API_KEY}" ]]; then
  echo "BATTERY_API_KEY is unset."
  exit 1
fi

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

CONTROLLER_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
BATTERY_PRINCIAL=$(dfx canister --network $IC_NETWORK id cycles_battery)

# Deploy tools canister
echo Deploying tools canister with owner=$CONTROLLER_PRINCIPAL on $IC_NETWORK, GOOGLE_API_KEY=$GOOGLE_API_KEY, GOOGLE_SEARCH_ENGINE_ID=$GOOGLE_SEARCH_ENGINE_ID, BATTERY_PRINCIAL=$BATTERY_PRINCIAL, BATTERY_API_KEY=$BATTERY_API_KEY on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$CONTROLLER_PRINCIPAL\", \"$GOOGLE_API_KEY\", \"$GOOGLE_SEARCH_ENGINE_ID\", opt \"$BATTERY_API_KEY\", opt principal \"$BATTERY_PRINCIAL\" )"

echo Tools Owner:
dfx canister --network $IC_NETWORK call arcmindai_tools get_owner
