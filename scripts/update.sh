# dfx identity use default
# NewPrincipal=$(dfx identity get-principal)
# echo NewPrincipal=$NewPrincipal

# dfx identity use icprod
# dfx canister call arcmindai_controller update_owner "(principal \"$NewPrincipal\")"
# dfx canister call arcmindai_controller get_owner

QUESTION1="I am Henry Chan. What is my first name?"

TODAY=$(date +"%Y-%m-%d")
QUESTION2="Today is $TODAY. I was born in 1988-08-16. How old am I?"

# dfx canister call arcmindai_controller insert_goal "(record {goal = \"$QUESTION2\"; status = variant {Scheduled = null}})"
# dfx canister call arcmindai_controller insert_goal "$QUESTION2"

RESULT="Your first name is Henry."
dfx canister call arcmindai_controller save_result "(1, \"$RESULT\")"
