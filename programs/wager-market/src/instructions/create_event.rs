use anchor_lang::prelude::*;
use std::mem::size_of;
use anchor_spl::token::TokenAccount;
use crate::state::{Event, EVENT_SIZE};
use crate::error::Error;
use crate::util::{assert_is_mint, is_default, make_ata};

#[derive(Accounts)]
#[instruction(id: [u8; 32])]
pub struct CreateEvent<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
    init,
    seeds = [b"event".as_ref(), id.as_ref()],
    bump,
    space = EVENT_SIZE,
    payer = authority,
    )]
    pub event: Box<Account<'info, Event>>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_event<'info>(
    ctx: Context<'_, '_, '_, 'info, CreateEvent<'info>>,
    id: [u8; 32],
    metadata_uri: String
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    event.authority = ctx.accounts.authority.key();
    event.id = id;
    event.metadata_uri = metadata_uri;

    // If there are remaining_accounts populated, we're using an alt currency mint
    if ctx.remaining_accounts.len() != 0 {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let currency_mint = next_account_info(remaining_accounts)?;
        let escrow_account = next_account_info(remaining_accounts)?;
        let token_program = next_account_info(remaining_accounts)?;
        let ata_program = next_account_info(remaining_accounts)?;

        assert_is_mint(currency_mint)?;
        event.currency_mint = currency_mint.key();

        // Create the escrow account for the token
        make_ata(
            escrow_account.to_account_info(),
            ctx.accounts.event.to_account_info(),
            currency_mint.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ata_program.to_account_info(),
            token_program.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            &[]
        )?;
    } else {
        event.currency_mint = Pubkey::default();
    }

    Ok(())
}