dfx canister call nns-ledger icrc1_transfer "(record { to = record { owner = principal \"aovwi-4maaa-aaaaa-qaagq-cai\";};  amount = 10_000_000_000;})"
dfx canister call nns-ledger icrc1_total_supply '()' 
dfx canister call nns-ledger icrc1_decimals '()' 

dfx canister call nns-ledger icrc1_balance_of "(record {owner = principal \"aovwi-4maaa-aaaaa-qaagq-cai\"; })"

dfx canister call nns-ledger icrc1_balance_of "(record {owner = principal \"hpikg-6exdt-jn33w-ndty3-fc7jc-tl2lr-buih3-cs3y7-tftkp-sfp62-gqe\"; })"

Account ID of principal o5uxz-byaaa-aaaah-adnyq-cai:
8419667c1541bcfb82c4ee1421a7f3cae62790f879d71c71e5546e867a339d2e


Please stream 0.01 ICP tokens to principal ID evdxs-el2wl-mrpaf-ehe75-uwoax-mqoc6-e55si-ls4va-gyecy-ywbio-bae
Please stream 0.01 ICP tokens to principal ID hpikg-6exdt-jn33w-ndty3-fc7jc-tl2lr-buih3-cs3y7-tftkp-sfp62-gqe


# Demo
# 1 ICP = 100_000_000
# 0.07731000 ICP = 7_731_000
# 0.05 = 5_000_000
dfx canister --network ic call nns-ledger icrc1_balance_of "(record {owner = principal \"wcz6z-niaaa-aaaah-adxzq-cai\"; })"
dfx canister --network ic call nns-ledger icrc1_transfer "(record { to = record { owner = principal \"wcz6z-niaaa-aaaah-adxzq-cai\";};  amount = 5_000_000;})"

# Please stream 0.01 ICP tokens to principal ID evdxs-el2wl-mrpaf-ehe75-uwoax-mqoc6-e55si-ls4va-gyecy-ywbio-bae