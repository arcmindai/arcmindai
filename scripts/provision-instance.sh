#!/bin/bash

# Validate required env vars
if [[ -z "${OPENAI_API_KEY}" ]]; then
  echo "OPENAI_API_KEY is unset."
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

if [[ -z "${OWNER_PRINCIPAL}" ]]; then
  echo "OWNER_PRINCIPAL is unset."
  exit 1
fi

if [[ -z "${BEAMFI_PRINCIPAL}" ]]; then
  echo "BEAMFI_PRINCIPAL is unset."
  exit 1
fi

if [[ -z "${BILLING_KEY}" ]]; then
  echo "BILLING_KEY is unset."
  exit 1
fi

if [[ -z "${BATTERY_API_KEY}" ]]; then
  echo "BATTERY_API_KEY is unset."
  exit 1
fi

if [[ -z "${BATTERY_PRINCIPAL}" ]]; then
  echo "BATTERY_PRINCIPAL is unset."
  exit 1
fi

GPT_MODEL=gpt-4o
BROWSE_WEBSITE_GPT_MODEL=gpt-4o

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}
echo Provisioning on $IC_NETWORK

# Create bare controller canister
echo Creating bare controller canister on $IC_NETWORK
dfx canister --network $IC_NETWORK create arcmindai_controller

CONTROLLER_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)

# Deploy vector canister
export CONTROLLER_PRINCIPAL=$CONTROLLER_PRINCIPAL
cd arcmindvector
pwd
VECTOR_PRINCIPAL=$(./scripts/provision.sh)

cd ../
pwd

# Deploy brain canister 
echo Deploying brain canister with owner $CONTROLLER_PRINCIPAL, GPT model $GPT_MODEL, openai_api_key $OPENAI_API_KEY, BATTERY_API_KEY=$BATTERY_API_KEY, BATTERY_PRINCIPAL=$BATTERY_PRINCIPAL on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_brain --argument "(opt principal \"$CONTROLLER_PRINCIPAL\", \"$OPENAI_API_KEY\", \"$GPT_MODEL\", opt \"$BATTERY_API_KEY\", opt principal \"$BATTERY_PRINCIPAL\")"

echo Brain Owner:
dfx canister --network $IC_NETWORK call arcmindai_brain get_owner

BRAIN_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_brain)

# Deploy tools canister
echo Deploying tools canister with owner $CONTROLLER_PRINCIPAL on $IC_NETWORK with GOOGLE_API_KEY=$GOOGLE_API_KEY, GOOGLE_SEARCH_ENGINE_ID=$GOOGLE_SEARCH_ENGINE_ID, BATTERY_API_KEY=$BATTERY_API_KEY, BATTERY_PRINCIPAL=$BATTERY_PRINCIPAL on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$CONTROLLER_PRINCIPAL\", \"$GOOGLE_API_KEY\", \"$GOOGLE_SEARCH_ENGINE_ID\", opt \"$BATTERY_API_KEY\", opt principal \"$BATTERY_PRINCIPAL\")"

echo Tools Owner:
dfx canister --network $IC_NETWORK call arcmindai_tools get_owner

TOOLS_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_tools)

# Deploy controller canister
echo Deploying controller canister with owner=$OWNER_PRINCIPAL, brain=$BRAIN_PRINCIPAL, browse_website_gpt_model=$BROWSE_WEBSITE_GPT_MODEL, tools=$TOOLS_PRINCIPAL, VECTOR_PRINCIPAL=$VECTOR_PRINCIPAL, BEAMFI_PRINCIPAL=$BEAMFI_PRINCIPAL, BATTERY_PRINCIPAL=$BATTERY_PRINCIPAL, BILLING_KEY=$BILLING_KEY, BATTERY_API_KEY=$BATTERY_API_KEY on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OWNER_PRINCIPAL\", opt principal \"$BRAIN_PRINCIPAL\", opt principal \"$TOOLS_PRINCIPAL\", opt principal \"$VECTOR_PRINCIPAL\", opt principal \"$BEAMFI_PRINCIPAL\", opt principal \"$BATTERY_PRINCIPAL\", opt \"$BROWSE_WEBSITE_GPT_MODEL\", opt \"$BILLING_KEY\", opt \"$BATTERY_API_KEY\")"

echo Controller Owner:
dfx canister --network $IC_NETWORK call arcmindai_controller get_owner

CONTROLL_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
echo "controller_principal=$CONTROLL_PRINCIPAL" >> $GITHUB_OUTPUT