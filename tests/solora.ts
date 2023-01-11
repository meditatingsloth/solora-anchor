import * as anchor from "@project-serum/anchor";
import { Solora } from "../target/types/solora";
import {LAMPORTS_PER_SOL, PublicKey, SYSVAR_RENT_PUBKEY} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from "crypto";
import {
	ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount,
	createMint, getAccount,
	getAssociatedTokenAddressSync, mintTo, NATIVE_MINT,
	TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import { v4 as uuidv4 } from 'uuid';
import moment from "moment";

describe("solora", async () => {

	const provider = anchor.AnchorProvider.env()
	provider.opts.skipPreflight = true
	anchor.setProvider(provider);

	const program = anchor.workspace.Solora as anchor.Program<Solora>;

	let eventId: number[];
	let metadataUri: string;
	let event: PublicKey;
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

	async function createEvent() {
		metadataUri = "https://example.com";
		eventId = Array.from(sha256(uuidv4()));

		[event] = PublicKey.findProgramAddressSync(
			[Buffer.from("event"), Buffer.from(eventId)],
			program.programId
		);

		const builder = program.methods.createEvent(
			eventId,
			feeAccount.publicKey,
			feeBps,
			new anchor.BN(0),
			metadataUri
		).accounts({
				payer: eventAuthority.publicKey,
				authority: eventAuthority.publicKey,
				event,
				systemProgram: anchor.web3.SystemProgram.programId,
				rent: anchor.web3.SYSVAR_RENT_PUBKEY,
			})
			.signers([eventAuthority])

		await builder.rpc();
	}

	async function createOrder(currencyMint=NATIVE_MINT, orderIndex=0, outcome=1, betAmount=LAMPORTS_PER_SOL, askBps=10000, expiry?: Date) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), numberToBuffer(orderIndex)],
			program.programId
		);

		const builder = program.methods.createOrder(
			outcome,
			new anchor.BN(betAmount),
			askBps,
			new anchor.BN(expiry ? Math.floor(expiry.getTime() / 1000) : 0),
		).accounts({
			authority: user.publicKey,
			order,
			event,
			systemProgram: anchor.web3.SystemProgram.programId,
			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
		}).signers([user])

		if (currencyMint != null &&
			currencyMint.toString() != NATIVE_MINT.toString()) {
			orderCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, order, true)
			userCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, user.publicKey)
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

		await builder.rpc();
	}

	async function fillOrder(orderIndex=0, outcome=2, fillAmount=LAMPORTS_PER_SOL, currencyMint=NATIVE_MINT) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), numberToBuffer(orderIndex)],
			program.programId
		);

		const builder = program.methods.fillOrder(
			orderIndex,
			outcome,
			new anchor.BN(fillAmount),
		).accounts({
			authority: userB.publicKey,
			order,
			event,
			systemProgram: anchor.web3.SystemProgram.programId,
			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
		}).signers([userB])

		if (currencyMint != null &&
			currencyMint.toString() != NATIVE_MINT.toString()) {
			orderCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, order, true)
			userBCurrencyAccount = getAssociatedTokenAddressSync(currencyMint, userB.publicKey)
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
			},{
				isWritable: false,
				isSigner: false,
				pubkey: TOKEN_PROGRAM_ID,
			}])
		}

		await builder.rpc();
	}

	async function settleEvent(outcome=1) {
		const builder = program.methods.settleEvent(eventId, outcome)
			.accounts({
				authority: eventAuthority.publicKey,
				event,
				systemProgram: anchor.web3.SystemProgram.programId,
			})
			.signers([eventAuthority])

		await builder.rpc();
	}

	function getFee(amount: number) {
		return Math.floor(amount * feeBps / 10000);
	}

	describe("ab create_event", function () {

		it("should create an event with correct values", async () => {
			await createEvent()

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.authority.toBase58(), eventAuthority.publicKey.toBase58());
			assert.equal(
				Buffer.from(fetchedEvent.id).toString('hex'),
				Buffer.from(eventId).toString('hex')
			);
			assert.equal(fetchedEvent.feeAccount.toBase58(), feeAccount.publicKey.toBase58());
			assert.equal(fetchedEvent.feeBps, feeBps);
			assert.equal(fetchedEvent.metadataUri, metadataUri);
		});

	});

	describe("create_order", function () {

		it("should create an order with correct values", async () => {
			await createEvent()
			await createOrder()

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.index, 0);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.outcome, 1);
			assert.equal(fetchedOrder.amount.toString(), LAMPORTS_PER_SOL.toString());
			assert.equal(fetchedOrder.currencyMint.toString(), NATIVE_MINT.toString());
			assert.equal(fetchedOrder.askBps, 10000);
			assert.equal(fetchedOrder.remainingAsk.toString(), LAMPORTS_PER_SOL.toString());
			assert.equal(fetchedOrder.expiry.toString(), '0');
			assert.deepEqual(fetchedOrder.fills, [])
		});

		it("should create an order with correct values for non-native token", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)

			await createEvent()
			await createOrder(currencyMint, 0, 1, 100)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.index, 0);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.outcome, 1);
			assert.equal(fetchedOrder.amount.toString(), '100');
			assert.equal(fetchedOrder.currencyMint.toString(), currencyMint.toString());
			assert.equal(fetchedOrder.askBps, 10000);
			assert.equal(fetchedOrder.expiry.toString(), '0');
			assert.deepEqual(fetchedOrder.fills, [])
		});

		it("should transfer user's lamports", async () => {
			await createEvent()

			const preBalance = await provider.connection.getBalance(user.publicKey)
			await createOrder()
			const postBalance = await provider.connection.getBalance(user.publicKey)

			assert.isAtMost(postBalance, preBalance - LAMPORTS_PER_SOL)
		});

		it("should set the correct expiry", async () => {
			await createEvent()
			const expiry = moment().add(1, 'day').toDate()
			await createOrder(NATIVE_MINT, 0, 1, LAMPORTS_PER_SOL, 10000, expiry)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.expiry.toString(), (Math.floor(expiry.getTime() / 1000)).toString())
		});

		it("should increment order index", async () => {
			await createEvent()
			await createOrder()

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.orderIndex, 1);

			await createOrder(NATIVE_MINT, 1)
			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.index, 1)
		});

		it("should transfer user's alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)

			await createEvent()
			await createOrder(currencyMint, 0, 1, 100, 10000, null)

			let userCurrencyAccountObject = await getAccount(provider.connection, userCurrencyAccount)
			assert.equal(userCurrencyAccountObject.amount.toString(), '0')

			let orderCurrencyAccountObject = await getAccount(provider.connection, orderCurrencyAccount)
			assert.equal(orderCurrencyAccountObject.amount.toString(), '100')
		});

		it("should throw when expiry is in the past", async () => {
			await createEvent()
			const expiry = moment().subtract(1, 'day').toDate()

			await assertThrows(async () =>
				await createOrder(NATIVE_MINT, 0, 1, LAMPORTS_PER_SOL, 10000, expiry),
				6004
			)
		});

		it("should throw when using the same order index", async () => {
			await createEvent()
			await createOrder()

			await assertThrows(async () => await createOrder(), 2006)
		});

	});

	describe("fill_order", function () {

		it("should fill an order with correct values", async () => {
			await createEvent()
			await createOrder()
			await fillOrder()

			let fetchedOrder = await program.account.order.fetch(order);
			assert.deepEqual(JSON.parse(JSON.stringify(fetchedOrder.fills)), [{
				index: 0,
				authority: userB.publicKey.toBase58(),
				outcome: 2,
				amount: new anchor.BN(LAMPORTS_PER_SOL).toString('hex'),
				isSettled: false
			}])
		});

		it("should transfer user's lamports", async () => {
			await createEvent()
			await createOrder()

			const preBalance = await provider.connection.getBalance(userB.publicKey)
			const orderPreBalance = await provider.connection.getBalance(order)
			await fillOrder()

			const postBalance = await provider.connection.getBalance(userB.publicKey)
			assert.isAtMost(postBalance, preBalance - LAMPORTS_PER_SOL)

			const orderPostBalance = await provider.connection.getBalance(order)
			assert.equal(orderPostBalance, orderPreBalance + LAMPORTS_PER_SOL)
		});

		it("should fill an order with alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)
			const userBCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, userB.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userBCurrencyAccount, payer.publicKey, 100)

			await createEvent()
			await createOrder(currencyMint, 0, 1, 100, 10000, null)
			await fillOrder(0, 2, 100, currencyMint)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.deepEqual(JSON.parse(JSON.stringify(fetchedOrder.fills)), [{
				index: 0,
				isSettled: false,
				authority: userB.publicKey.toBase58(),
				outcome: 2,
				amount: new anchor.BN(100).toString('hex'),
			}])
		});

		it("should transfer user's alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)
			const userBCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, userB.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userBCurrencyAccount, payer.publicKey, 100)

			await createEvent()
			await createOrder(currencyMint, 0, 1, 100, 10000, null)
			await fillOrder(0, 2, 100, currencyMint)

			let userBAccount = await getAccount(provider.connection, userBCurrencyAccount)
			assert.equal(userBAccount.amount.toString(), '0')

			let escrowAccount = await getAccount(provider.connection, orderCurrencyAccount)
			// 100 from the order, 100 from the fill
			assert.equal(escrowAccount.amount.toString(), '200')
		});

		it("should throw an error when filling with outcome 0", async function () {
			await createEvent()
			await createOrder()

			await assertThrows(async() => {
				await fillOrder(0, 0)
			}, 6001)
		});

		it("should throw an error when event is already settled", async function () {
			await createEvent()
			await createOrder()

			await assertThrows(async() => {
				await settleEvent()
				await fillOrder()
			}, 6000)
		});

		it("should throw an error when choosing the same outcome as order", async function () {
			await createEvent()
			await createOrder()

			await assertThrows(async() => {
				await fillOrder(0, 1, LAMPORTS_PER_SOL)
			}, 6001)
		});

		it("should throw an error when the order has expired", async function () {
			await createEvent()
			const expiry = moment().add(3, 'seconds').toDate()
			await createOrder(NATIVE_MINT, 0, 1, LAMPORTS_PER_SOL, 10000, expiry)

			await new Promise(resolve => setTimeout(resolve, 4000))

			await assertThrows(async() => {
				await fillOrder()
			}, 6005)
		});

		it("should throw an error when fill amount is too large", async function () {
			await createEvent()
			await createOrder()

			await assertThrows(async() => {
				await fillOrder(0, 2, LAMPORTS_PER_SOL * 2)
			}, 6003)
		});

	});

});
