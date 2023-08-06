# To run this script, set following Env Vars in your terminal or uncomment below to set it
# OPENAI_API_KEY="UPDATE_ME"
echo OPENAI_API_KEY = $OPENAI_API_KEY

OwnerPrincipal=$(dfx identity get-principal)

# Deploy brain canister 
echo Deploying brain canister with owner $OwnerPrincipal and openai_api_key $OPENAI_API_KEY
dfx deploy arcmindai_brain --argument "(opt principal \"$OwnerPrincipal\", \"$OPENAI_API_KEY\")"

BrainPrincipal=$(dfx canister id arcmindai_brain)

# Deploy controller canister
echo Deploying controller canister with owner $OwnerPrincipal and brain $BrainPrincipal
dfx deploy arcmindai_controller --argument "(opt principal \"$OwnerPrincipal\", opt principal \"$BrainPrincipal\")"
echo Controller Owner:
dfx canister call arcmindai_controller get_owner

# Update brain owner to controller
ControllerPrincipal=$(dfx canister id arcmindai_controller)
echo Updating brain owner to controller $ControllerPrincipal
dfx canister call arcmindai_brain update_owner "(principal \"$ControllerPrincipal\")"

echo Brain Owner:
dfx canister call arcmindai_brain get_owner