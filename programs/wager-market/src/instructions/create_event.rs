use anchor_lang::prelude::*;
use std::mem::size_of;
use crate::state::{Event, EVENT_SIZE};
use crate::error::Error;
use crate::util::{assert_is_mint, is_default};

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

    if ctx.remaining_accounts.len() != 0 {
        let remaining_accounts = &mut ctx.remaining_accounts.iter();
        let currency_mint = next_account_info(remaining_accounts)?;
        assert_is_mint(currency_mint)?;
        event.currency_mint = currency_mint.key();
    } else {
        event.currency_mint = Pubkey::default();
    }

    Ok(())
}