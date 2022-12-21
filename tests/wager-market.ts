import * as anchor from "@project-serum/anchor";
import { WagerMarket } from "../target/types/wager_market";
import {LAMPORTS_PER_SOL, PublicKey} from "@solana/web3.js";
import {
	TOKEN_PROGRAM_ID,
	getAccount,
	getOrCreateAssociatedTokenAccount
} from "@solana/spl-token";
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
	let bump: number;

	const payer = anchor.web3.Keypair.generate();
	const authority = anchor.web3.Keypair.generate();
	const user = anchor.web3.Keypair.generate();
	const userB = anchor.web3.Keypair.generate();

	before(async () => {
		await Promise.all([payer, authority, user, userB].map(keypair => {
			return provider.connection.requestAirdrop(keypair.publicKey, 10 * LAMPORTS_PER_SOL).then(sig =>
				provider.connection.confirmTransaction(sig, "processed")
			)
		}))
	})

	function sha256(str: string) {
		return crypto.createHash('sha256').update(str).digest();
	}

	async function createEvent() {
		metadataUri = "https://example.com";
		eventId = Array.from(sha256("Test event"));

		[event, bump] = PublicKey.findProgramAddressSync(
			[Buffer.from("event"), Buffer.from(eventId)],
			program.programId
		);

		await program.methods.createEvent(eventId, metadataUri)
			.accounts({
				authority: authority.publicKey,
				event,
				systemProgram: anchor.web3.SystemProgram.programId,
				rent: anchor.web3.SYSVAR_RENT_PUBKEY,
			}).signers([authority]).rpc();
	}

	describe("create_event", function () {

		it("ab should create and event with correct values", async () => {
			await createEvent()

			let fetchedEvent = await program.account.event.fetch(event);
			assert.equal(fetchedEvent.authority.toBase58(), authority.publicKey.toBase58());
			assert.equal(
				Buffer.from(fetchedEvent.id).toString('hex'),
				Buffer.from(eventId).toString('hex')
			);
			assert.equal(fetchedEvent.metadataUri, metadataUri);
		});

	});
	
});
