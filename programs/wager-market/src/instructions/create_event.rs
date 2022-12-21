use anchor_lang::prelude::*;
use std::mem::size_of;
use crate::state::{Event, EVENT_SIZE};
use crate::error::Error;

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

    Ok(())
}