use anchor_lang::prelude::*;

#[derive(
	AnchorSerialize,
	AnchorDeserialize,
	Clone,
	Copy,
	PartialEq,
	Debug
)]
pub enum Outcome {
	Undrawn,
	Invalid,
	Up,
	Down,
}