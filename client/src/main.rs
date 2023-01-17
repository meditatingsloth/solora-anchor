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
use solora_pyth_price::state::EventConfig;

fn main() -> ClientResult<()> {
    // Creating a Client with your default paper keypair as payer
    let client = default_client();
    client.airdrop(&client.payer_pubkey(), 1 * LAMPORTS_PER_SOL)?;

    let (event_config, next_event_pubkey) = create_event_config(&client)?;

    create_event(&client, event_config, next_event_pubkey)?;

    Ok(())
}

fn create_event_config(client: &Client) -> ClientResult<(EventConfig, Pubkey)> {
    let payer = client.payer_pubkey();
    // SOL/USD price feed: https://pyth.network/price-feeds/crypto-sol-usd?cluster=mainnet-beta
    // copied account to test validator using https://book.anchor-lang.com/anchor_references/anchor-toml_reference.html#testvalidatorclone
    let pyth_feed = Pubkey::from_str("J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix").unwrap();

    let auth_seeds = [
        b"event_config".as_ref(),
        payer.as_ref(),
        pyth_feed.as_ref(),
        spl_token::native_mint::ID.as_ref(),
    ];
    let event_config_pubkey = Pubkey::find_program_address(&auth_seeds, &solora_pyth_price::ID).0;
    let event_config_account = client.get_account(&event_config_pubkey)?;
    if !event_config_account.data.is_empty() {
        // Already exists
        let event_config = solora_pyth_price::state::EventConfig::try_from(&event_config_account.data)?;
        let auth_seeds = [
            b"event".as_ref(),
            event_config_pubkey.as_ref(),
            event_config.next_event_start.as_ref(),
        ];
        let event_pubkey = Pubkey::find_program_address(&auth_seeds, &solora_pyth_price::ID).0;
        return Ok((event_config, event_pubkey));
    }

    let create_event_thread = Thread::pubkey(event_config_pubkey, "event_create".into());

    println!(
        "create event thread: 🔗 {}",
        explorer().thread_url(create_event_thread, thread_program_ID)
    );

    let next_event_start = Utc::now().timestamp() + 60;
    let create_event_config_ix = Instruction {
        program_id: solora_pyth_price::ID,
        accounts: vec![
            AccountMeta::new(client.payer_pubkey(), true),
            AccountMeta::new(event_config_pubkey, false),
            AccountMeta::new_readonly(pyth_feed, false),
            AccountMeta::new_readonly(client.payer_pubkey(), false),
            AccountMeta::new_readonly(spl_token::native_mint::ID, false),
            AccountMeta::new(create_event_thread, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data: solora_pyth_price::instruction::CreateEventConfig {
            interval_seconds: 60,
            next_event_start
        }.data(),
    };

    sign_send_and_confirm_tx(
        &client,
        [create_event_config_ix].to_vec(),
        None,
        "create_event_config".into(),
    )?;

    let event_config_account = client.get_account(&event_config_pubkey)?;
    let mut data: &[u8] = &event_config_account.data;
    let event_config = ???
    let auth_seeds = [
        b"event".as_ref(),
        event_config_pubkey.as_ref(),
        next_event_start.as_ref(),
    ];
    let event_pubkey = Pubkey::find_program_address(&auth_seeds, &solora_pyth_price::ID).0;

    Ok((event_config, event_pubkey))
}

fn create_event(client: &Client, event_config: EventConfig, event_pubkey: Pubkey) -> ClientResult<()> {
    let payer = client.payer_pubkey();

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
            AccountMeta::new(event_config.key(), false),
            AccountMeta::new(event_pubkey, false),
            AccountMeta::new_readonly(event_config.pyth_feed, false),
            AccountMeta::new_readonly(client.payer_pubkey(), false),
            AccountMeta::new_readonly(event_config.currency_mint, false),
            AccountMeta::new(lock_thread_pubkey, false),
            AccountMeta::new(settle_thread_pubkey, false),
            AccountMeta::new_readonly(thread_program_ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        data: solora_pyth_price::instruction::CreateEvent{
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