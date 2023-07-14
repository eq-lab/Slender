#!/bin/bash

source .$NODE_ENV.env

TREASURY_SECRET="${TREASURY_SECRET:-SDUQ5O67BWZUAR6GTCFWN6QM7BZBXDD5SRPMWUTIS2N37CVNKPNV3GFY}"
TREASURY_PUBLIC="${TREASURY_PUBLIC:-GCG4IJLJBBHAAACKB245CSY6HFDQDL3OO4FCNYACQE2S7X4P36FAXT3Q}"

curl -s "$FRIENDBOT_URL?addr=$TOKEN_PUBLIC" 1>/dev/null
curl -s "$FRIENDBOT_URL?addr=$POOL_PUBLIC" 1>/dev/null
curl -s "$FRIENDBOT_URL?addr=$USER_PUBLIC" 1>/dev/null
sleep 10

deploy() {
    local address=$(soroban contract deploy \
        --wasm $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE")
    echo $address
}

install() {
    local hash=$(soroban contract install \
        --wasm $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE")
    echo $hash
}

invoke() {
    local result=$(soroban contract invoke \
        --id $1 \
        --source $2 \
        --rpc-url "$SOROBAN_RPC_URL" \
        --network-passphrase "$PASSPHRASE" \
        -- \
        $3)
    echo $result
}

addressFromResult() {
    IFS=',' read -ra values <<<"$(sed 's/\[\|\]//g' <<<"$1")"
    local value1="$(echo "${values[0]}" | tr -d '[:space:]' | sed 's/"//g' | sed 's/\[//g')"
    local value2="$(echo "${values[1]}" | tr -d '[:space:]')"

    echo $value1
}

TOKEN=$(deploy "../contracts/soroban_token_contract.wasm" $TOKEN_SECRET)
echo "Token contract address: $TOKEN"
invoke $TOKEN $TOKEN_SECRET "initialize \
    --admin $TOKEN_PUBLIC \
    --decimal 9 \
    --name $(echo -n "token" | xxd -p) \
    --symbol $(echo -n "TKN" | xxd -p)"

DEBT_TOKEN=$(deploy "../contracts/soroban_token_contract.wasm" $TOKEN_SECRET)
echo "Token contract address (debt token): $DEBT_TOKEN"
invoke $DEBT_TOKEN $TOKEN_SECRET "initialize \
    --admin $TOKEN_PUBLIC \
    --decimal 9 \
    --name $(echo -n "debt-token" | xxd -p) \
    --symbol $(echo -n "DTKN" | xxd -p)"

deployer=$(deploy "../target/wasm32-unknown-unknown/release/deployer.wasm" $TOKEN_SECRET)
echo "Deployer contract address: $deployer"

stokenHash=$(install "../target/wasm32-unknown-unknown/release/s_token.wasm" $TOKEN_SECRET)
echo "S-token wasm hash: $stokenHash"

poolHash=$(install "../target/wasm32-unknown-unknown/release/pool.wasm" $TOKEN_SECRET)
echo "Pool wasm hash: $poolHash"

deployPoolResult=$(invoke $deployer $POOL_SECRET "deploy_pool \
    --salt 0000000000000000000000000000000000000000000000000000000000000000 \
    --wasm_hash $poolHash \
    --admin $POOL_PUBLIC")
POOL=$(addressFromResult $deployPoolResult)
echo "Pool contract address: $POOL"

deployStokenResult=$(invoke $deployer $TOKEN_SECRET "deploy_s_token \
    --salt 0000000000000000000000000000000000000000000000000000000000000001 \
    --wasm_hash $stokenHash \
    --decimal 9 \
    --name $(echo -n "stoken" | xxd -p) \
    --symbol $(echo -n "STKN" | xxd -p) \
    --pool $POOL \
    --treasury $TREASURY_PUBLIC \
    --underlying_asset $TOKEN")
STOKEN=$(addressFromResult $deployStokenResult)
echo "Stoken contract address: $STOKEN"

priceFeed=$(deploy "../target/wasm32-unknown-unknown/release/price_feed_mock.wasm" $POOL_SECRET)
PRICE_FEED=$(addressFromResult $priceFeed)
echo "Price feed contract address: $PRICE_FEED"

contracts=".contracts"
{
    echo "POOL=$POOL"
    echo "STOKEN=$STOKEN"
    echo "TOKEN=$TOKEN"
    echo "DEBT_TOKEN=$DEBT_TOKEN"
    echo "PRICE_FEED=$PRICE_FEED"
} >$contracts

echo "Contracts deployed"