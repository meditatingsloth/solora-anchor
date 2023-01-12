use anchor_spl::token::spl_token;
use chrono::Utc;
use solana_sdk::sysvar;
use {
    anchor_lang::{prelude::*, solana_program::system_program, InstructionData},
    clockwork_client::{
        thread::{
            ID as thread_program_ID,
            state::{Thread},
        },
        Client, ClientResult,
    },
    clockwork_utils::{explorer::Explorer},
    solana_sdk::{
        instruction::Instruction, native_token::LAMPORTS_PER_SOL,
        signature::Keypair, signature::read_keypair_file,
        transaction::Transaction,
    },
    std::str::FromStr,
};

fn main() -> ClientResult<()> {
    // Creating a Client with your default paper keypair as payer
    let client = default_client();
    client.airdrop(&client.payer_pubkey(), 1 * LAMPORTS_PER_SOL)?;

    // create thread that listens for account changes for a pyth pricing feed
    create_event(&client)?;

    Ok(())
}

fn create_event(client: &Client) -> ClientResult<()> {
    // SOL/USD price feed: https://pyth.network/price-feeds/crypto-sol-usd?cluster=mainnet-beta
    // copied account to test validator using https://book.anchor-lang.com/anchor_references/anchor-toml_reference.html#testvalidatorclone
    let pyth_feed = Pubkey::from_str("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix").unwrap();
    let lock_time = Utc::now().timestamp() + 60; // 60 seconds from now
    let lock_time_bytes = lock_time.to_le_bytes();
    let payer = client.payer_pubkey();
    let auth_seeds = [
        b"event".as_ref(),
        pyth_feed.as_ref(),
        payer.as_ref(),
        spl_token::native_mint::ID.as_ref(),
        lock_time_bytes.as_ref(),
    ];
    let event_pubkey = Pubkey::find_program_address(&auth_seeds, &solora_pyth_price::ID).0;
    let lock_thread_pubkey = Thread::pubkey(event_pubkey, "event_lock".into());

    println!(
        "lock thread: 🔗 {}",
        explorer().thread_url(lock_thread_pubkey, thread_program_ID)
    );

    let settle_thread_pubkey = Thread::pubkey(event_pubkey, "event_settle".into());
    println!(
        "settle thread: 🔗 {}",
        explorer().thread_url(settle_thread_pubkey, thread_program_ID)
    );

    let create_event_ix = Instruction {
        program_id: solora_pyth_price::ID,
        accounts: vec![
            AccountMeta::new(client.payer_pubkey(), true),
            AccountMeta::new_readonly(client.payer_pubkey(), false),
            AccountMeta::new(event_pubkey, false),
            AccountMeta::new_readonly(pyth_feed, false),
            AccountMeta::new_readonly(client.payer_pubkey(), false),
            AccountMeta::new_readonly(spl_token::native_mint::ID, false),
            AccountMeta::new(lock_thread_pubkey, false),
            AccountMeta::new(settle_thread_pubkey, false),
            AccountMeta::new_readonly(thread_program_ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data: solora_pyth_price::instruction::CreateEvent{
            lock_time,
            wait_period: 60,
            fee_bps: 300
        }.data(),
    };

    sign_send_and_confirm_tx(
        &client,
        [create_event_ix].to_vec(),
        None,
        "create_event".into(),
    )?;
    Ok(())
}

pub fn sign_send_and_confirm_tx(
    client: &Client,
    ix: Vec<Instruction>,
    signers: Option<Vec<&Keypair>>,
    label: String,
) -> ClientResult<()> {
    let mut tx;

    match signers {
        Some(signer_keypairs) => {
            tx = Transaction::new_signed_with_payer(
                &ix,
                Some(&client.payer_pubkey()),
                &signer_keypairs,
                client.get_latest_blockhash().unwrap(),
            );
        }
        None => {
            tx = Transaction::new_with_payer(&ix, Some(&client.payer_pubkey()));
        }
    }

    tx.sign(&[client.payer()], client.latest_blockhash().unwrap());

    // Send and confirm tx
    match client.send_and_confirm_transaction(&tx) {
        Ok(sig) => println!(
            // Eventually also use EXPLORER.clockwork instead of EXPLORER.solana, so ppl don't have to use two explorers
            "{} tx: ✅ {}",
            label,
            explorer().tx_url(sig)
        ),
        Err(err) => println!("{} tx: ❌ {:#?}", label, err),
    }
    Ok(())
}

fn explorer() -> Explorer {
    #[cfg(feature = "localnet")]
        return Explorer::custom("http://localhost:8899".to_string());
    #[cfg(not(feature = "localnet"))]
        Explorer::devnet()
}

fn default_client() -> Client {
    let host = "https://api.devnet.solana.com";
    let config_file = solana_cli_config::CONFIG_FILE.as_ref().unwrap().as_str();
    let config = solana_cli_config::Config::load(config_file).unwrap();
    let payer = read_keypair_file(format!("../{}", &config.keypair_path)).unwrap();
    Client::new(payer, host.into())
}