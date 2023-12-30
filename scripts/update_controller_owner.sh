#!/bin/bash

# Validate required env vars
if [[ -z "${CONTROLLER_OWNER}" ]]; then
  echo "CONTROLLER_OWNER is unset."
  exit 1
fi

if [[ -z "${CONTROLLER_CANISTER_ID}" ]]; then
  echo "CONTROLLER_CANISTER_ID is unset."
  exit 1
fi

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

echo Updating controller canister $CONTROLLER_CANISTER_ID owner to $CONTROLLER_OWNER
dfx canister --network $IC_NETWORK call $CONTROLLER_CANISTER_ID update_owner "(principal \"$CONTROLLER_OWNER\")"

echo controller Owner:
dfx canister --network $IC_NETWORK call $CONTROLLER_CANISTER_ID get_owner
