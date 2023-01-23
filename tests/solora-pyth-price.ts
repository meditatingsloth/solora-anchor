import * as anchor from "@project-serum/anchor";
import { SoloraPythPrice } from "../target/types/solora_pyth_price";
import { Pyth } from "../target/types/pyth";
import {LAMPORTS_PER_SOL, PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from "crypto";
import {
	ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount,
	createMint, getAccount,
	getAssociatedTokenAddressSync, mintTo, NATIVE_MINT,
	TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import moment from "moment";
import {mockOracle} from "./pythHelpers";

/**
 * TODO: These tests are outdated from before Pyth/Clockwork was introduced. To run tests
 * with pyth/clockwork integration 2 things need to be done:
 * 1. Update pythHelpers with serialization/deserialization of Pyth 0.7.0 structs
 * 2. Run clockwork locally before running tests
 */

describe("solora-pyth-price", () => {

	const provider = anchor.AnchorProvider.env()
	provider.opts.skipPreflight = true
	anchor.setProvider(provider);

	const program = anchor.workspace.SoloraPythPrice as anchor.Program<SoloraPythPrice>;
	const defaultWaitPeriod = 1

	let event: PublicKey;
	let pythFeed: PublicKey;
	let order: PublicKey;
	let orderCurrencyAccount: PublicKey;
	let userCurrencyAccount: PublicKey;
	let userBCurrencyAccount: PublicKey;

	let eventAuthority = anchor.web3.Keypair.generate();
	const payer = anchor.web3.Keypair.generate();
	const user = anchor.web3.Keypair.generate();
	const userB = anchor.web3.Keypair.generate();
	let feeAccount = anchor.web3.Keypair.generate();
	let feeBps: number;
	let pythPrice: number;
	let lockTime: number;

	before(async () => {
		await Promise.all([payer, eventAuthority, user, userB].map(keypair => {
			return provider.connection.requestAirdrop(keypair.publicKey, 100 * LAMPORTS_PER_SOL).then(sig =>
				provider.connection.confirmTransaction(sig, "processed")
			)
		}))
	})

	beforeEach(setUpData)

	async function setUpData() {
		feeBps = 300
		pythPrice = 50 * 10**5
	}

	function sha256(str: string) {
		return crypto.createHash('sha256').update(str).digest();
	}

	function numberToBuffer(num: number) {
		const buf = Buffer.alloc(4);
		buf.writeUInt32LE(num);
		return buf;
	}

	async function assertThrows(fn: () => Promise<any | void>, code?: number, message?: string) {
		let throws = false
		try {
			await fn()
		} catch (e) {
			console.log(`[${e.code ?? ''}] ${e.message}`)
			throws = true
			if (code) {
				throws = e.code === code
			}
			if (message) {
				throws = e.message.includes(message)
			}
		}
		assert.isTrue(throws, 'Expected error to be thrown')
	}

	async function createEvent(currencyMint = NATIVE_MINT, secondsUntilLock = 3, waitPeriod = defaultWaitPeriod) {
		pythFeed = await mockOracle(
			payer,
			pythPrice,
			-5,
			100
		);

		lockTime = moment().unix() + secondsUntilLock;

		[event] = PublicKey.findProgramAddressSync(
			[
				Buffer.from("event"),
				pythFeed.toBuffer(),
				feeAccount.publicKey.toBuffer(),
				currencyMint.toBuffer(),
				new anchor.BN(lockTime).toBuffer('le', 8),
			],
			program.programId
		);

		const builder = program.methods.createEvent(
			new anchor.BN(lockTime),
			waitPeriod,
			feeBps,
		).accounts({
			payer: eventAuthority.publicKey,
			authority: eventAuthority.publicKey,
			event,
			pythFeed,
			feeAccount: feeAccount.publicKey,
			currencyMint,
		})
			.signers([eventAuthority])

		await builder.rpc();
	}

	async function createOrder(payer = user, currencyMint = NATIVE_MINT, outcome, betAmount = LAMPORTS_PER_SOL) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), payer.publicKey.toBuffer()],
			program.programId
		);

		const builder = program.methods.createOrder(
			outcome,
			new anchor.BN(betAmount),
		).accounts({
			authority: payer.publicKey,
			order,
			event
		}).signers([payer])

		if (currencyMint !== null &&
			currencyMint.toString() !== NATIVE_MINT.toString()) {
			orderCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, order, true)
			userCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, payer.publicKey)
			builder.remainingAccounts([{
				isWritable: false,
				isSigner: false,
				pubkey: currencyMint,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: orderCurrencyAccount,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: userCurrencyAccount,
			}, {
				isWritable: false,
				isSigner: false,
				pubkey: TOKEN_PROGRAM_ID,
			}, {
				isWritable: false,
				isSigner: false,
				pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
			}, {
				isWritable: false,
				isSigner: false,
				pubkey: SYSVAR_RENT_PUBKEY,
			}])
		}
		try {
			await builder.rpc();
		}
		catch (err) {
			console.log(err);
		}
	}

	async function setLockPrice() {
		await program.methods.setLockPrice()
			.accounts({
				authority: eventAuthority.publicKey,
				event,
				pythFeed,
				systemProgram: SystemProgram.programId,
			})
			.signers([eventAuthority])
			.rpc();
	}

	async function settleOrder(payer = user, currencyMint = NATIVE_MINT) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), payer.publicKey.toBuffer()],
			program.programId
		);

		const builder = program.methods.settleOrder().accounts({
			authority: payer.publicKey,
			order,
			event,
			systemProgram: anchor.web3.SystemProgram.programId,
			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
		}).signers([payer])

		if (currencyMint != null &&
			currencyMint.toString() != NATIVE_MINT.toString()) {
			orderCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, order, true)
			userBCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, payer.publicKey)
			builder.remainingAccounts([{
				isWritable: false,
				isSigner: false,
				pubkey: currencyMint,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: orderCurrencyAccount,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: userBCurrencyAccount,
			}, {
				isWritable: false,
				isSigner: false,
				pubkey: TOKEN_PROGRAM_ID,
			}])
		}

		await builder.rpc();
	}

	async function settleEvent() {
		const builder = program.methods.settleEvent()
			.accounts({
				authority: eventAuthority.publicKey,
				event,
				pythFeed,
			})
			.signers([eventAuthority])

		await builder.rpc();
	}

	function getFee(amount: number) {
		return Math.floor(amount * feeBps / 10000);
	}

	describe("create_event", function () {

		it("should create an event with correct values", async () => {
			await createEvent()

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.authority.toBase58(), eventAuthority.publicKey.toBase58());
			assert.equal(fetchedEvent.pythFeed.toBase58(), pythFeed.toBase58());
			assert.equal(fetchedEvent.feeAccount.toBase58(), feeAccount.publicKey.toBase58());
			assert.equal(fetchedEvent.feeBps, feeBps);
			assert.notEqual(fetchedEvent.lockTime.toString(), '0');
			assert.equal(fetchedEvent.waitPeriod, 1);
			assert.equal(fetchedEvent.currencyMint.toBase58(), NATIVE_MINT.toBase58());
			assert.equal(Object.keys(fetchedEvent.outcome)[0], 'undrawn');
		});

	});

	describe("create_order", function () {

		it("should create an order with correct values", async () => {
			await createEvent()
			await createOrder(user, NATIVE_MINT, { up: {} })

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(Object.keys(fetchedOrder.outcome)[0], 'up');
			assert.equal(fetchedOrder.amount.toString(), LAMPORTS_PER_SOL.toString());
		});

		it("should create an order with correct values for non-native token", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)

			await createEvent(currencyMint)
			await createOrder(user, currencyMint, { up: {} }, 100)

			let fetchedOrder = await program.account.order.fetch(order);
			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.amount.toString(), '100');
			assert.equal(Object.keys(fetchedOrder.outcome)[0], 'up');
			assert.equal(fetchedEvent.upAmount.toString(), '100');
			assert.equal(fetchedEvent.upCount, 1);
		});

		it("should transfer user's lamports", async () => {
			await createEvent()

			const preBalance = await provider.connection.getBalance(user.publicKey)
			await createOrder(user, NATIVE_MINT, { up: {} })
			const postBalance = await provider.connection.getBalance(user.publicKey)

			assert.isAtMost(postBalance, preBalance - LAMPORTS_PER_SOL)
		});

		it("should transfer user's alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)

			await createEvent(currencyMint)
			await createOrder(user, currencyMint, { up: {} }, 100);

			let userCurrencyAccountObject = await getAccount(provider.connection, userCurrencyAccount)
			assert.equal(userCurrencyAccountObject.amount.toString(), '0')

			let orderCurrencyAccountObject = await getAccount(provider.connection, orderCurrencyAccount)
			assert.equal(orderCurrencyAccountObject.amount.toString(), '100')
		});

		it("should create both types of orders", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)
			const userBCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, userB.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userBCurrencyAccount, payer.publicKey, 69)

			await createEvent(currencyMint)
			await createOrder(user, currencyMint, { up: {} }, 100)
			await createOrder(userB, currencyMint, { down: {} }, 69)

			const [orderA] = PublicKey.findProgramAddressSync(
				[Buffer.from("order"), event.toBuffer(), user.publicKey.toBuffer()],
				program.programId
			);

			const [orderB] = PublicKey.findProgramAddressSync(
				[Buffer.from("order"), event.toBuffer(), userB.publicKey.toBuffer()],
				program.programId
			);

			let fetchedOrder = await program.account.order.fetch(orderA);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58(), 'incorrect authority');
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.amount.toString(), '100');
			assert.equal(Object.keys(fetchedOrder.outcome)[0], 'up');
			let fetchedOrderB = await program.account.order.fetch(orderB);
			assert.equal(fetchedOrderB.authority.toBase58(), userB.publicKey.toBase58());
			assert.equal(fetchedOrderB.event.toString(), event.toString());
			assert.equal(fetchedOrderB.amount.toString(), '69');
			assert.equal(Object.keys(fetchedOrderB.outcome)[0], 'down');
			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.upAmount.toString(), '100');
			assert.equal(fetchedEvent.upCount, 1);
			assert.equal(fetchedEvent.downAmount.toString(), '69');
			assert.equal(fetchedEvent.downCount, 1);
		});

	});

	describe("set_lock_price", function () {

		it("ab should set the lock price", async () => {
			await createEvent(NATIVE_MINT, 2, 0)
			await new Promise(resolve => setTimeout(resolve, 2500));
			await setLockPrice()

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal('undrawn', Object.keys(fetchedEvent.outcome)[0]);
			assert.equal(pythPrice.toString(), fetchedEvent.lockPrice.toString());
		});

	});

	describe("settle_event", function () {
		it("should settle event", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)
			const userBCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, userB.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userBCurrencyAccount, payer.publicKey, 69)

			await createEvent(currencyMint)
			await createOrder(user, currencyMint, { up: {} }, 100)
			await createOrder(userB, currencyMint, { down: {} }, 69)
			await settleEvent();
			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.upAmount.toString(), '100');
			assert.equal(fetchedEvent.upCount, 1);
			assert.equal(fetchedEvent.downAmount.toString(), '69');
			assert.equal(fetchedEvent.downCount, 1);
			assert.equal(Object.keys(fetchedEvent.outcome)[0], 'up');
		})
	})

	describe("settle_order", function () {

		// it("should fill an order with correct values", async () => {
		// 	await createEvent()
		// 	await createOrder()
		// 	await settleEvent()
		// 	await settleOrder()
		// });

		// it("should transfer user's lamports", async () => {
		// 	await createEvent()
		// 	await createOrder()

		// 	const preBalance = await provider.connection.getBalance(userB.publicKey)
		// 	const orderPreBalance = await provider.connection.getBalance(order)
		// 	await fillOrder()

		// 	const postBalance = await provider.connection.getBalance(userB.publicKey)
		// 	assert.isAtMost(postBalance, preBalance - LAMPORTS_PER_SOL)

		// 	const orderPostBalance = await provider.connection.getBalance(order)
		// 	assert.equal(orderPostBalance, orderPreBalance + LAMPORTS_PER_SOL)
		// });

		// it("should fill an order with alt currency", async () => {
		// 	const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
		// 	const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
		// 	await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)
		// 	const userBCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, userB.publicKey)
		// 	await mintTo(provider.connection, payer, currencyMint, userBCurrencyAccount, payer.publicKey, 100)

		// 	await createEvent()
		// 	await createOrder(currencyMint, 0, 1, 100, 10000, null)
		// 	await fillOrder(0, 2, 100, currencyMint)

		// 	let fetchedOrder = await program.account.order.fetch(order);
		// 	assert.deepEqual(JSON.parse(JSON.stringify(fetchedOrder.fills)), [{
		// 		index: 0,
		// 		isSettled: false,
		// 		authority: userB.publicKey.toBase58(),
		// 		outcome: 2,
		// 		amount: new anchor.BN(100).toString('hex'),
		// 	}])
		// });

		// it("should transfer user's alt currency", async () => {
		// 	const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
		// 	const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
		// 	await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)
		// 	const userBCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, userB.publicKey)
		// 	await mintTo(provider.connection, payer, currencyMint, userBCurrencyAccount, payer.publicKey, 100)

		// 	await createEvent()
		// 	await createOrder(currencyMint, 0, 1, 100, 10000, null)
		// 	await fillOrder(0, 2, 100, currencyMint)

		// 	let userBAccount = await getAccount(provider.connection, userBCurrencyAccount)
		// 	assert.equal(userBAccount.amount.toString(), '0')

		// 	let escrowAccount = await getAccount(provider.connection, orderCurrencyAccount)
		// 	// 100 from the order, 100 from the fill
		// 	assert.equal(escrowAccount.amount.toString(), '200')
		// });

		// it("should throw an error when filling with outcome 0", async function () {
		// 	await createEvent()
		// 	await createOrder()

		// 	await assertThrows(async () => {
		// 		await fillOrder(0, 0)
		// 	}, 6001)
		// });

		// it("should throw an error when event is already settled", async function () {
		// 	await createEvent()
		// 	await createOrder()

		// 	await assertThrows(async () => {
		// 		await settleEvent()
		// 		await fillOrder()
		// 	}, 6000)
		// });

		// it("should throw an error when choosing the same outcome as order", async function () {
		// 	await createEvent()
		// 	await createOrder()

		// 	await assertThrows(async () => {
		// 		await fillOrder(0, 1, LAMPORTS_PER_SOL)
		// 	}, 6001)
		// });

		// it("should throw an error when the order has expired", async function () {
		// 	await createEvent()
		// 	const expiry = moment().add(3, 'seconds').toDate()
		// 	await createOrder(NATIVE_MINT, 0, 1, LAMPORTS_PER_SOL, 10000, expiry)

		// 	await new Promise(resolve => setTimeout(resolve, 4000))

		// 	await assertThrows(async () => {
		// 		await fillOrder()
		// 	}, 6005)
		// });

		// it("should throw an error when fill amount is too large", async function () {
		// 	await createEvent()
		// 	await createOrder()

		// 	await assertThrows(async () => {
		// 		await fillOrder(0, 2, LAMPORTS_PER_SOL * 2)
		// 	}, 6003)
		// });

	});

});
