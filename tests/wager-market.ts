import * as anchor from "@project-serum/anchor";
import { WagerMarket } from "../target/types/wager_market";
import {LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";
import { assert } from "chai";
import * as crypto from "crypto";

describe("wager-market", async () => {

	const provider = anchor.AnchorProvider.env()
	provider.opts.skipPreflight = true
	anchor.setProvider(provider);

	const program = anchor.workspace.WagerMarket as anchor.Program<WagerMarket>;

	let eventId: number[];
	let metadataUri: string;
	let event: PublicKey;
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

	async function assertThrows(fn: () => Promise<any>, code?: number, message?: string) {
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
		eventId = Array.from(sha256("Test event"));

		[event] = PublicKey.findProgramAddressSync(
			[Buffer.from("event"), Buffer.from(eventId)],
			program.programId
		);

		await program.methods.createEvent(eventId, metadataUri)
			.accounts({
				authority: eventAuthority.publicKey,
				event,
				systemProgram: anchor.web3.SystemProgram.programId,
				rent: anchor.web3.SYSVAR_RENT_PUBKEY,
			}).signers([eventAuthority]).rpc();
	}

	async function createOrder(outcome=1, betAmount=LAMPORTS_PER_SOL, askBps=10000) {
		[order] = PublicKey.findProgramAddressSync(
			[Buffer.from("order"), event.toBuffer(), user.publicKey.toBuffer()],
			program.programId
		);

		await program.methods.createOrder(
			outcome,
			new anchor.BN(betAmount),
			askBps
		).accounts({
			authority: user.publicKey,
			order,
			event,
			systemProgram: anchor.web3.SystemProgram.programId,
			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
		}).signers([user]).rpc();
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
		});

	});

	describe("create_order", function () {

		it("should create an order with correct values", async () => {
			await createEvent()

			const beforeBalance = await provider.connection.getBalance(user.publicKey)
			await createOrder()
			const afterBalance = await provider.connection.getBalance(user.publicKey)
			console.log(afterBalance - beforeBalance)

			let fetchedOrder = await program.account.order.fetch(order);
			assert.equal(fetchedOrder.authority.toBase58(), user.publicKey.toBase58());
			assert.equal(fetchedOrder.event.toString(), event.toString());
			assert.equal(fetchedOrder.outcome, 1);
			assert.equal(fetchedOrder.betAmount.toString(), LAMPORTS_PER_SOL.toString());
			assert.equal(fetchedOrder.askBps, 10000);
			assert.deepEqual(fetchedOrder.fills, [])
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
