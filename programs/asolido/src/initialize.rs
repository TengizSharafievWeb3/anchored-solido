use anchor_lang::prelude::*;
use std::collections::BTreeMap;

use crate::metrics::Metrics;
use crate::state::{ExchangeRate, FeeRecipients, Maintainers, Validators, LIDO_CONSTANT_SIZE};
use crate::{Initialize, RewardDistribution};

impl<'info> Initialize<'info> {
    pub fn process(
        &mut self,
        bumps: &BTreeMap<String, u8>,
        version: u8,
        reward_distribution: RewardDistribution,
        max_validators: u32,
        max_maintainers: u32,
    ) -> Result<()> {
        let lido = &mut self.lido;

        lido.lido_version = version;
        lido.manager = self.manager.key();
        lido.exchange_rate = ExchangeRate::default();
        lido.sol_reserve_account_bump_seed = *bumps.get("reserve").unwrap();
        lido.mint_authority_bump_seed = *bumps.get("mint_authority").unwrap();
        lido.stake_authority_bump_seed = *bumps.get("stake_authority").unwrap();
        lido.rewards_withdraw_authority_bump_seed =
            *bumps.get("rewards_withdraw_authority").unwrap();
        lido.reward_distribution = reward_distribution;
        lido.fee_recipients = FeeRecipients {
            treasury_account: self.treasury.key(),
            developer_account: self.developer.key(),
        };
        lido.metrics = Metrics::new();
        lido.maintainers = Maintainers::new(max_maintainers);
        lido.validators = Validators::new(max_validators);

        Ok(())
    }

    /// Return how many bytes are needed to serialize an Lido for validators and maintainers numbers
    pub fn required_bytes(max_validators: u32, max_maintainers: u32) -> usize {
        // Bytes required for maintainers
        let bytes_for_maintainers = Maintainers::required_bytes(max_maintainers as usize);
        // Bytes required for validators
        let bytes_for_validators = Validators::required_bytes(max_validators as usize);
        // Calculate the expected lido's size
        8 + LIDO_CONSTANT_SIZE + bytes_for_validators + bytes_for_maintainers
    }
}
