MyPrincipal=$(dfx identity get-principal)

# Deploy the controller canister
echo Deploying controller canister with principal $MyPrincipal
dfx deploy arcmindai_controller --argument "(opt principal \"$MyPrincipal\")"
echo Controller Owner:
dfx canister call arcmindai_controller get_owner

# Deploy the brain canister with controller as owner
ControllerPrincipal=$(dfx canister id arcmindai_controller)
echo Deploying brain canister with controller principal $ControllerPrincipal
dfx deploy arcmindai_brain --argument "(opt principal \"$ControllerPrincipal\")"

echo Brain Owner:
dfx canister call arcmindai_brain get_owner