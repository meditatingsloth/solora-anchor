#!/usr/local/bin/bash

PROGRAM_NAME="solora-pyth-price"

solana config set --url https://api.devnet.solana.com
anchor build
solana program deploy -k ./keys/admin.json ./target/deploy/$PROGRAM_NAME.so
anchor idl upgrade SPPQ71aSCVntCUUWYxpkQa6awxpfgWsuU13H4WTwEAG -f ./target/idl/$PROGRAM_NAME.json --provider.cluster devnet --provider.wallet ./keys/admin.json