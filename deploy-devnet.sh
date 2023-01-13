#!/usr/local/bin/bash

PROGRAM_NAME="solora_pyth_price"
PROGRAM="SPPBWBa5ooYYTVZDT7ESkU8ui7uiUjok6YBNeDogbx5"
DEPLOYER="DPLiu1NuyaFKUbf87Wes4tE85jXT18NS4MpptGbksAVR"

solana config set --url https://api.devnet.solana.com
anchor build
cp ./target/idl/${PROGRAM_NAME}.json ../solora-ui/src/idls/${PROGRAM_NAME}.json
solana program deploy -k ./keys/${DEPLOYER}.json ./target/deploy/$PROGRAM_NAME.so
anchor idl upgrade ${PROGRAM} -f ./target/idl/$PROGRAM_NAME.json --provider.cluster devnet --provider.wallet ./keys/${DEPLOYER}.json