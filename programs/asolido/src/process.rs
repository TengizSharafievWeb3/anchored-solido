use anchor_lang::prelude::*;
use std::collections::BTreeMap;

use crate::maintainers::Maintainers;
use crate::metrics::Metrics;
use crate::state::{ExchangeRate, FeeRecipients, LIDO_CONSTANT_SIZE};
use crate::validators::Validators;
use crate::{Deposit, Initialize, Lamports, RewardDistribution, LidoError};
use crate::logic::mint_st_sol_to;

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
        lido.st_sol_mint = self.st_sol_mint.key();
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

impl<'info> Deposit<'info> {
    pub fn process(&mut self, amount: Lamports) -> Result<()> {
        require!(amount.amount > 0, LidoError::InvalidAmount);

        let cpi_accounts = anchor_lang::system_program::Transfer {
            from: self.user.to_account_info(),
            to: self.reserve.to_account_info(),
        };
        let cpi_context = anchor_lang::context::CpiContext::new(
            self.system_program.to_account_info(), cpi_accounts
        );
        anchor_lang::system_program::transfer(cpi_context, amount.amount)?;

        let st_sol_amount = self.lido.exchange_rate.exchange_sol(amount)?;

        mint_st_sol_to(&self.lido,
        self.token_program.to_account_info(),
            self.st_sol_mint.to_account_info(),
            self.mint_authority.to_account_info(),
            self.recipient.to_account_info(),
            st_sol_amount
        )?;

        // TODO: emit event about deposit

        self.lido.metrics.observe_deposit(amount)?;

        Ok(())
    }
}
