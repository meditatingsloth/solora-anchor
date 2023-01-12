#!/usr/local/bin/bash

PROGRAM_NAME="solora_pyth_price"

solana config set --url https://west.sentries.io/2ceb6a385b424fa38ef3482f4a31e9c9
anchor build
solana program deploy -k ./keys/admin.json ./target/deploy/$PROGRAM_NAME.so
anchor idl upgrade SPPQ71aSCVntCUUWYxpkQa6awxpfgWsuU13H4WTwEAG -f ./target/idl/$PROGRAM_NAME.json --provider.cluster mainnet --provider.wallet ./keys/admin.json