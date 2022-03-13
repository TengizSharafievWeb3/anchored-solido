use anchor_lang::prelude::*;

use crate::{Initialize, RewardDistribution};

impl<'info> Initialize<'info> {
    pub fn process(
        &mut self,
        version: u8,
        reward_distribution: RewardDistribution,
        max_validators: u32,
        max_maintainers: u32,
    ) -> Result<()> {
        Ok(())
    }
}
