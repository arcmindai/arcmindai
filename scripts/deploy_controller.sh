# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}

OwnerPrincipal=$(dfx identity --network $IC_NETWORK get-principal)
BrainPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_brain)
ToolsPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_tools)
BROWSE_WEBSITE_GPT_MODEL=gpt-3.5-turbo-16k

# Deploy controller canister
echo Deploying controller canister with owner $OwnerPrincipal, brain $BrainPrincipal, browse_website_gpt_model $BROWSE_WEBSITE_GPT_MODEL and tools $ToolsPrincipal on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_controller --argument "(opt principal \"$OwnerPrincipal\", opt principal \"$BrainPrincipal\", opt principal \"$ToolsPrincipal\", opt \"$BROWSE_WEBSITE_GPT_MODEL\")"
