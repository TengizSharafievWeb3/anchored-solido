use anchor_lang::prelude::*;
use crate::RewardDistribution;

pub const LIDO_VERSION: u8 = 0;

#[account]
#[derive(Default)]
pub struct Lido {}

#[account]
#[derive(Default)]
pub struct Reserve {}