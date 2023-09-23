# Solora

![](https://dc100lan3jpki.cloudfront.net/images/Black%20w_%20Gradient%20Logo%20Twitter.png)

Solora is a prediction market built on the Solana blockchain.

The price prediction game is built using the clockwork.xyz on-chain automation engine to lock and settle events, and Pyth oracles are used for retrieving asset prices.

The `solora-pyth-price` program is an open program that allows anyone to set up their own price prediction game with configurable time intervals, fees, and fee burning. Native SOL and SPL tokens can be used as the betting currency.

To play, users predict whether the price of the asset from the configured Pyth oracle will go up or down in the next interval of time and place a bet. All bets are pooled together and winners get their share of the total based on their bet size minus the configured fee percentage. Please note: a manual crank is still required to create new events.

![](https://dc100lan3jpki.cloudfront.net/images/solora_xyz.jpg)

### Program Address:
devnet: `SPPq79wtPSBeFvYJbSxS9Pj1JdbQARDWxwJBXyTVcRg`

## Other Programs
The `solora-order-book` program is a work in progress and is not yet ready for use. This program allows users to place bets with odds that can be accepted by others in a P2P fashion.

The `pyth` program is meant to be used in tests to mock Pyth prices.
