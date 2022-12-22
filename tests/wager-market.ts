import * as anchor from "@project-serum/anchor";
import { WagerMarket } from "../target/types/wager_market";
import {LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from "crypto";
import {
	ASSOCIATED_TOKEN_PROGRAM_ID, createAssociatedTokenAccount,
	createMint, getAccount,
	getAssociatedTokenAddress,
	getAssociatedTokenAddressSync, mintTo,
	TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import { v4 as uuidv4 } from 'uuid';
import moment from "moment";

describe("wager-market", async () => {

	const provider = anchor.AnchorProvider.env()
	provider.opts.skipPreflight = true
	anchor.setProvider(provider);

	const program = anchor.workspace.WagerMarket as anchor.Program<WagerMarket>;

	let eventId: number[];
	let metadataUri: string;
	let event: PublicKey;
	let escrow: PublicKey;
	let order: PublicKey;

	const payer = anchor.web3.Keypair.generate();
	const eventAuthority = anchor.web3.Keypair.generate();
	const user = anchor.web3.Keypair.generate();
	const userB = anchor.web3.Keypair.generate();

	before(async () => {
		await Promise.all([payer, eventAuthority, user, userB].map(keypair => {
			return provider.connection.requestAirdrop(keypair.publicKey, 10 * LAMPORTS_PER_SOL).then(sig =>
				provider.connection.confirmTransaction(sig, "processed")
			)
		}))
	})

	function sha256(str: string) {
		return crypto.createHash('sha256').update(str).digest();
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

	async function createEvent(currencyMint?: PublicKey) {
		metadataUri = "https://example.com";
		eventId = Array.from(sha256(uuidv4()));

		[event] = PublicKey.findProgramAddressSync(
			[Buffer.from("event"), Buffer.from(eventId)],
			program.programId
		);

		const builder = program.methods.createEvent(eventId, metadataUri)
			.accounts({
				authority: eventAuthority.publicKey,
				event,
				systemProgram: anchor.web3.SystemProgram.programId,
				rent: anchor.web3.SYSVAR_RENT_PUBKEY,
			})
			.signers([eventAuthority])

		if (currencyMint) {
			escrow = getAssociatedTokenAddressSync(currencyMint, event, true)
			builder.remainingAccounts([{
				isWritable: false,
				isSigner: false,
				pubkey: currencyMint,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: escrow,
			}, {
				isWritable: false,
				isSigner: false,
				pubkey: TOKEN_PROGRAM_ID,
			}, {
				isWritable: false,
				isSigner: false,
				pubkey: ASSOCIATED_TOKEN_PROGRAM_ID,
			}])
		}

		await builder.rpc();
	}

	async function createOrder(outcome=1, betAmount=LAMPORTS_PER_SOL, askBps=10000, expiry?: Date, currencyMint?: PublicKey) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), user.publicKey.toBuffer()],
			program.programId
		);

		const builder = program.methods.createOrder(
			outcome,
			new anchor.BN(betAmount),
			askBps,
			expiry ? new anchor.BN(Math.floor(expiry.getTime() / 1000)) : null,
		).accounts({
			authority: user.publicKey,
			order,
			event,
			systemProgram: anchor.web3.SystemProgram.programId,
			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
		}).signers([user])

		if (currencyMint) {
			escrow = getAssociatedTokenAddressSync(currencyMint, event, true)
			builder.remainingAccounts([{
				isWritable: false,
				isSigner: false,
				pubkey: currencyMint,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: escrow,
			}, {
				isWritable: true,
				isSigner: false,
				pubkey: getAssociatedTokenAddressSync(currencyMint, user.publicKey),
			},{
				isWritable: false,
				isSigner: false,
				pubkey: TOKEN_PROGRAM_ID,
			}])
		}

		await builder.rpc();
	}

	async function fillOrder(outcome=0, fillAmount=LAMPORTS_PER_SOL) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), user.publicKey.toBuffer()],
			program.programId
		);

		await program.methods.fillOrder(
			outcome,
			new anchor.BN(fillAmount),
		).accounts({
			authority: userB.publicKey,
			order,
			event,
			systemProgram: anchor.web3.SystemProgram.programId,
			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
		}).signers([userB]).rpc();
	}

	async function settleEvent() {

	}

	describe("create_event", function () {

		it("should create an event with correct values", async () => {
			await createEvent()

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.authority.toBase58(), eventAuthority.publicKey.toBase58());
			assert.equal(
				Buffer.from(fetchedEvent.id).toString('hex'),
				Buffer.from(eventId).toString('hex')
			);
			assert.equal(fetchedEvent.metadataUri, metadataUri);
			assert.equal(fetchedEvent.currencyMint.toBase58(), PublicKey.default.toBase58());
		});

		it("should create an event with alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			await createEvent(currencyMint)

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.authority.toBase58(), eventAuthority.publicKey.toBase58());
			assert.equal(
				Buffer.from(fetchedEvent.id).toString('hex'),
				Buffer.from(eventId).toString('hex')
			);
			assert.equal(fetchedEvent.metadataUri, metadataUri);
			assert.equal(fetchedEvent.currencyMint.toBase58(), currencyMint.toBase58());
		});

		it("should create escrow with alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			await createEvent(currencyMint)

			const escrowAccount = await getAccount(provider.connection, escrow)
			assert.isTrue(escrowAccount.isInitialized)
		});

	});

	describe("create_order", function () {

		it("should create an order with correct values", async () => {
			await createEvent()
			await createOrder()

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.outcome, 1);
			assert.equal(fetchedOrder.betAmount.toString(), LAMPORTS_PER_SOL.toString());
			assert.equal(fetchedOrder.askBps, 10000);
			assert.equal(fetchedOrder.expiry, new anchor.BN(-1));
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
			await createOrder(1, LAMPORTS_PER_SOL, 10000, expiry)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.expiry.toString(), (Math.floor(expiry.getTime() / 1000)).toString())
		});

		it("should create an order with alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)

			await createEvent(currencyMint)
			await createOrder(1, 100, 10000, null, currencyMint)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.outcome, 1);
			assert.equal(fetchedOrder.betAmount.toString(), '100');
			assert.equal(fetchedOrder.askBps, 10000);
			assert.equal(fetchedOrder.expiry, new anchor.BN(-1));
			assert.deepEqual(fetchedOrder.fills, [])
		});

		it("should transfer user's alt currency", async () => {
			const currencyMint = await createMint(provider.connection, payer, payer.publicKey, payer.publicKey, 0)
			const userCurrencyAccount = await createAssociatedTokenAccount(provider.connection, payer, currencyMint, user.publicKey)
			await mintTo(provider.connection, payer, currencyMint, userCurrencyAccount, payer.publicKey, 100)

			await createEvent(currencyMint)
			await createOrder(1, 100, 10000, null, currencyMint)

			let userAccount = await getAccount(provider.connection, userCurrencyAccount)
			assert.equal(userAccount.amount.toString(), '0')

			let escrowAccount = await getAccount(provider.connection, escrow)
			assert.equal(escrowAccount.amount.toString(), '100')
		});

		it("should throw when expiry is in the past", async () => {
			await createEvent()
			const expiry = moment().subtract(1, 'day').toDate()

			await assertThrows(async () =>
				await createOrder(1, LAMPORTS_PER_SOL, 10000, expiry),
				6004
			)
		});

	});

	describe("fill_order", function () {

		it("should fill an order with correct values", async () => {
			await createEvent()
			await createOrder()

			const beforeBalance = await provider.connection.getBalance(userB.publicKey)
			await fillOrder()
			const afterBalance = await provider.connection.getBalance(userB.publicKey)
			assert.isAtLeast(beforeBalance - afterBalance, 200000)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.outcome, 1);
			assert.equal(fetchedOrder.betAmount.toString(), LAMPORTS_PER_SOL.toString());
			assert.equal(fetchedOrder.askBps, 10000);
			assert.deepEqual(JSON.parse(JSON.stringify(fetchedOrder.fills)), [{
				authority: userB.publicKey.toBase58(),
				outcome: 0,
				fillAmount: new anchor.BN(LAMPORTS_PER_SOL).toString('hex'),
			}])
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
				await fillOrder(1, LAMPORTS_PER_SOL)
			}, 6001)
		});

		it("should throw an error when fill amount is too large", async function () {
			await createEvent()
			await createOrder()

			await assertThrows(async() => {
				await fillOrder(0, LAMPORTS_PER_SOL * 2)
			}, 6003)
		});

	});

});
