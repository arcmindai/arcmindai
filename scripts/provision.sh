MyPrincipal=$(dfx identity get-principal)

echo Deploying controller canister with principal $MyPrincipal
dfx deploy arcmindai_controller --argument "(opt principal \"$MyPrincipal\")"
echo Controller Owner:
dfx canister call arcmindai_controller get_owner

ControllerPrincipal=$(dfx canister id arcmindai_controller)
echo Deploying brain canister with controller principal $ControllerPrincipal
dfx deploy arcmindai_brain --argument "(opt principal \"$ControllerPrincipal\")"

echo Brain Owner:
dfx canister call arcmindai_brain get_owner