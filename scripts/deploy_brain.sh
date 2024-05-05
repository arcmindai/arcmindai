#!/bin/bash

# Validate required env vars
if [[ -z "${OPENAI_API_KEY}" ]]; then
  echo "OPENAI_API_KEY is unset."
  exit 1
fi

if [[ -z "${BATTERY_API_KEY}" ]]; then
  echo "BATTERY_API_KEY is unset."
  exit 1
fi

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}
GPT_MODEL=gpt-4

# Deploy brain canister 
CONTROLLER_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
BATTERY_PRINCIPAL=$(dfx canister --network $IC_NETWORK id cycles_battery)

echo Deploying brain canister with owner=$CONTROLLER_PRINCIPAL, GPT model=$GPT_MODEL and OPENAI_API_KEY=$OPENAI_API_KEY, BATTERY_PRINCIPAL=$BATTERY_PRINCIPAL, BATTERY_API_KEY=$BATTERY_API_KEY on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_brain --argument "(opt principal \"$CONTROLLER_PRINCIPAL\", \"$OPENAI_API_KEY\", \"$GPT_MODEL\", opt \"$BATTERY_API_KEY\", opt principal \"$BATTERY_PRINCIPAL\")"
