# To deplopy locally, update IC_NETWORK to local. To deploy to ic, update IC_NETWORK to ic.
IC_NETWORK=local
echo Asking on $IC_NETWORK

# dfx canister call arcmindai_controller ask '("Henry")'
# dfx canister call arcmindai_brain ask '("I am Henry Chan. What is my first name?")'
# dfx canister call arcmindai_controller ask '("I am Henry Chan. What is my first name?")'

# TODAY=$(date +"%Y-%m-%d")
# QUESTION="Today is $TODAY. I was born in 1988-08-16. How old am I?"

QUESTION="I am Henry Chan. What is my first name?"
echo Questions: $QUESTION

dfx canister --network $IC_NETWORK call arcmindai_controller ask "(\"$QUESTION\")"