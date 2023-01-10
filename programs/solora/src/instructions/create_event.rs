use anchor_lang::prelude::*;
use crate::state::{Event, EVENT_SIZE};

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
    fee_account: Pubkey,
    fee_bps: u32,
    metadata_uri: String
) -> Result<()> {
    let event = &mut ctx.accounts.event;
    event.bump = [*ctx.bumps.get("event").unwrap()];
    event.authority = ctx.accounts.authority.key();
    event.id = id;
    event.fee_account = fee_account;
    event.fee_bps = fee_bps;
    event.metadata_uri = metadata_uri;

    Ok(())
}