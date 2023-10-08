# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=local

OWENR_PRINCIPAL=$(dfx identity --network $IC_NETWORK get-principal)

# Deploy tools canister
echo Deploying tools canister with owner $OWENR_PRINCIPAL on $IC_NETWORK, GOOGLE_API_KEY=$GOOGLE_API_KEY, GOOGLE_SEARCH_ENGINE_ID=$GOOGLE_SEARCH_ENGINE_ID
dfx deploy --network $IC_NETWORK arcmindai_tools --argument "(opt principal \"$OWENR_PRINCIPAL\", \"$GOOGLE_API_KEY\", \"$GOOGLE_SEARCH_ENGINE_ID\")"
