# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=${IC_NETWORK:-local}
GPT_MODEL=gpt-4

# Deploy brain canister 
echo OPENAI_API_KEY = $OPENAI_API_KEY
ControllerPrincipal=$(dfx canister --network $IC_NETWORK id arcmindai_controller)
echo Deploying brain canister with owner=$ControllerPrincipal, GPT model=$GPT_MODEL and OPENAI_API_KEY=$OPENAI_API_KEY on $IC_NETWORK
dfx deploy --network $IC_NETWORK arcmindai_brain --argument "(opt principal \"$ControllerPrincipal\", \"$OPENAI_API_KEY\", \"$GPT_MODEL\")"
