# Update controller owner

# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

CONTROLLER_OWNER=og5ck-wstph-2m7s5-enjzf-7heh5-ko754-eoaj4-n2ccx-imtiu-s4ieb-4ae
echo Updating controller owner to controller $CONTROLLER_OWNER
dfx canister --network $IC_NETWORK call arcmindai_controller update_owner "(principal \"$CONTROLLER_OWNER\")"

echo controller Owner:
dfx canister --network $IC_NETWORK call arcmindai_controller get_owner
