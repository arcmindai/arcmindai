#!/bin/bash

# Validate required env vars
if [[ -z "${CYCLES_BATTERY_API_KEY}" ]]; then
  echo "CYCLES_BATTERY_API_KEY is unset."
  exit 1
fi

if [[ -z "${OWNER_PRINCIPAL}" ]]; then
  echo "OWNER_PRINCIPAL is unset."
  exit 1
fi

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

# Deploy cycles battery canister
echo Deploying cycles battery canister with owner=$OWNER_PRINCIPAL api_key=$CYCLES_BATTERY_API_KEY on $IC_NETWORK
dfx deploy --network $IC_NETWORK cycles_battery --argument "(opt principal \"$OWNER_PRINCIPAL\", opt \"$CYCLES_BATTERY_API_KEY\")"
