#!/usr/local/bin/bash

PROGRAM_NAME="solora_pyth_price"
PROGRAM="SPPq79wtPSBeFvYJbSxS9Pj1JdbQARDWxwJBXyTVcRg"
DEPLOYER="DPLiu1NuyaFKUbf87Wes4tE85jXT18NS4MpptGbksAVR"

solana config set --url https://api.devnet.solana.com
anchor build -p $PROGRAM_NAME
cp ./target/idl/${PROGRAM_NAME}.json ../solora-monorepo/packages/solora-ts/src/${PROGRAM_NAME}.json
solana program deploy -k ./keys/${DEPLOYER}.json ./target/deploy/$PROGRAM_NAME.so

if [ "$1" = "idl-init" ]
then
  anchor idl init ${PROGRAM} -f ./target/idl/$PROGRAM_NAME.json --provider.cluster devnet --provider.wallet ./keys/${DEPLOYER}.json
fi

if [ "$1" = "idl-upgrade" ]
then
  anchor idl upgrade ${PROGRAM} -f ./target/idl/$PROGRAM_NAME.json --provider.cluster devnet --provider.wallet ./keys/${DEPLOYER}.json
fi