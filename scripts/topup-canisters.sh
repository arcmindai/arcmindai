# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

# Validate required env vars
if [[ -z "${CYCLES_BATTERY_API_KEY}" ]]; then
  echo "CYCLES_BATTERY_API_KEY is unset."
  exit 1
fi

GROUP_NAME="test"
REQ_CYCLES_AMOUNT=1000000
CYCLES_BALANCE=3100000000000


echo Calling cycles battery canister with GROUP_NAME=$GROUP_NAME CYCLES_BATTERY_API_KEY=$CYCLES_BATTERY_API_KEY, REQ_CYCLES_AMOUNT=$REQ_CYCLES_AMOUNT, CYCLES_BALANCE=$CYCLES_BALANCE on $IC_NETWORK
dfx canister --network $IC_NETWORK call cycles_battery topup_cycles "(\"$GROUP_NAME\", \"$CYCLES_BATTERY_API_KEY\", $REQ_CYCLES_AMOUNT, $CYCLES_BALANCE)"