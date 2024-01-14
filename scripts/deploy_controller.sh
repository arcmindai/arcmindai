#!/bin/bash

# Validate required env vars
if [[ -z "${VECTOR_PRINCIPAL}" ]]; then
  echo "VECTOR_PRINCIPAL is unset."
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

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

BRAIN_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_brain)
TOOLS_PRINCIPAL=$(dfx canister --network $IC_NETWORK id arcmindai_tools)
BROWSE_WEBSITE_GPT_MODEL=gpt-3.5-turbo-1106

# Deploy controller canister
echo Deploying controller canister with owner=$OWNER_PRINCIPAL, brain=$BRAIN_PRINCIPAL, browse_website_gpt_model=$BROWSE_WEBSITE_GPT_MODEL, tools=$TOOLS_PRINCIPAL, VECTOR_PRINCIPAL=$VECTOR_PRINCIPAL, BEAMFI_PRINCIPAL=$BEAMFI_PRINCIPAL on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OWNER_PRINCIPAL\", opt principal \"$BRAIN_PRINCIPAL\", opt principal \"$TOOLS_PRINCIPAL\", opt principal \"$VECTOR_PRINCIPAL\", opt principal \"$BEAMFI_PRINCIPAL\", opt \"$BROWSE_WEBSITE_GPT_MODEL\")"
