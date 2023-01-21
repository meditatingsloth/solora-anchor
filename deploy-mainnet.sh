#!/usr/local/bin/bash

PROGRAM_NAME="solora_pyth_price"
PROGRAM="SPPq79wtPSBeFvYJbSxS9Pj1JdbQARDWxwJBXyTVcRg"
DEPLOYER="DPLiu1NuyaFKUbf87Wes4tE85jXT18NS4MpptGbksAVR"

solana config set --url https://rpc.helius.xyz?api-key=3cf96e28-5ae3-4f9d-8d08-df4bb56cd769
anchor build -p $PROGRAM_NAME
cp ./target/idl/${PROGRAM_NAME}.json ../solora-monorepo/packages/solora-ts/src/${PROGRAM_NAME}.json
solana program deploy -k ./keys/${DEPLOYER}.json ./target/deploy/$PROGRAM_NAME.so
#anchor idl upgrade ${PROGRAM} -f ./target/idl/$PROGRAM_NAME.json --provider.cluster mainnet --provider.wallet ./keys/${DEPLOYER}.json