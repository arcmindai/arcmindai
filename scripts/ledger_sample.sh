dfx canister call nns-ledger icrc1_transfer "(record { to = record { owner = principal \"aovwi-4maaa-aaaaa-qaagq-cai\";};  amount = 10_000_000_000;})"
dfx canister call nns-ledger icrc1_total_supply '()' 
dfx canister call nns-ledger icrc1_decimals '()' 

dfx canister call nns-ledger icrc1_balance_of "(record {owner = principal \"aovwi-4maaa-aaaaa-qaagq-cai\"; })"

dfx canister call nns-ledger icrc1_balance_of "(record {owner = principal \"hpikg-6exdt-jn33w-ndty3-fc7jc-tl2lr-buih3-cs3y7-tftkp-sfp62-gqe\"; })"
