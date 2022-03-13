// SPDX-FileCopyrightText: 2021 Chorus One AG
// SPDX-License-Identifier: GPL-3.0

//! State transition types


use std::ops::Range;
use anchor_lang::prelude::*;
use solana_program::clock::Epoch;
use crate::account_map::{AccountMap, AccountSet, EntryConstantSize, PubkeyAndEntry};
use crate::error::LidoError;
use crate::metrics::Metrics;
use crate::token;
use crate::token::{Lamports, Rational, StLamports};

pub const LIDO_VERSION: u8 = 0;
pub const VALIDATOR_CONSTANT_SIZE: usize = 89;


pub type Validators = AccountMap<Validator>;

impl Validators {
    pub fn iter_active(&self) -> impl Iterator<Item = &Validator> {
        self.iter_entries().filter(|&v| v.active)
    }

    pub fn iter_active_entries(&self) -> impl Iterator<Item = &PubkeyAndEntry<Validator>> {
        self.entries.iter().filter(|&v| v.entry.active)
    }
}
pub type Maintainers = AccountSet;

impl EntryConstantSize for Validator {
    const SIZE: usize = VALIDATOR_CONSTANT_SIZE;
}

impl EntryConstantSize for () {
    const SIZE: usize = 0;
}

/// The exchange rate used for deposits and rewards distribution.
///
/// The exchange rate of SOL to stSOL is determined by the SOL balance of
/// Solido, and the total stSOL supply: every stSOL represents a share of
/// ownership of the SOL pool.
///
/// Deposits do not change the exchange rate: we mint new stSOL proportional to
/// the amount deposited, to keep the exchange rate constant. However, rewards
/// *do* change the exchange rate. This is how rewards get distributed to stSOL
/// holders without any transactions: their stSOL will be worth more SOL.
///
/// Let's call an increase of the SOL balance that mints a proportional amount
/// of stSOL a *deposit*, and an increase of the SOL balance that does not mint
/// any stSOL a *donation*. The ordering of donations relative to one another is
/// not relevant, and the order of deposits relative to one another is not
/// relevant either. But the order of deposits relative to donations is: if you
/// deposit before a donation, you get more stSOL than when you deposit after.
/// If you deposit before, you benefit from the reward, if you deposit after,
/// you do not. In formal terms, *deposit and and donate do not commute*.
///
/// This presents a problem if we want to do rewards distribution in multiple
/// steps (one step per validator). Reward distribution is a combination of a
/// donation (the observed rewards minus fees), and a deposit (the fees, which
/// get paid as stSOL). Because deposit and donate do not commute, different
/// orders of observing validator rewards would lead to different outcomes. We
/// don't want that.
///
/// To resolve this, we use a fixed exchange rate, and update it once per epoch.
/// This means that a donation no longer changes the exchange rate (not
/// instantly at least). That means that we can observe validator rewards in any
/// order we like. A different way of thinking about this, is that by fixing
/// the exchange rate for the duration of the epoch, all the different ways of
/// ordering donations and deposits have the same outcome, so every sequence of
/// deposits and donations is equivalent to one where they all happen
/// simultaneously at the start of the epoch. Time within an epoch ceases to
/// exist, the only thing relevant is the epoch.
///
/// When we update the exchange rate, we set the values to the balance that we
/// inferred by tracking all changes. This does not include any external
/// modifications (validation rewards paid into stake accounts) that were not
/// yet observed at the time of the update.
///
/// When we observe the actual validator balance in `WithdrawInactiveStake`, the
/// difference between the tracked balance and the observed balance, is a
/// donation that will be returned to the reserve account.
///
/// We collect the rewards accumulated by a validator with the
/// `CollectValidatorFee` instruction. This function distributes the accrued
/// rewards paid to the Solido program (as we enforce that 100% of the fees goes
/// to the Solido program).
///
/// `CollectValidatorFee` is blocked in a given epoch, until we update the
/// exchange rate in that epoch. Validation rewards are distributed at the start
/// of the epoch. This means that in epoch `i`:
///
/// 1. `UpdateExchangeRate` updates the exchange rate to what it was at the end
///    of epoch `i - 1`.
/// 2. `CollectValidatorFee` runs for every validator, and observes the
///    rewards. Deposits (including those for fees) in epoch `i` therefore use
///    the exchange rate at the end of epoch `i - 1`, so deposits in epoch `i`
///    do not benefit from rewards received in epoch `i`.
/// 3. Epoch `i + 1` starts, and validation rewards are paid into validator's
/// vote accounts.
/// 4. `UpdateExchangeRate` updates the exchange rate to what it was at the end
///    of epoch `i`. Everybody who deposited in epoch `i` (users, as well as fee
///    recipients) now benefit from the validation rewards received in epoch `i`.
/// 5. Etc.
#[derive(
Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, Eq, PartialEq,
)]
pub struct ExchangeRate {
    /// The epoch in which we last called `UpdateExchangeRate`.
    pub computed_in_epoch: Epoch,

    /// The amount of stSOL that existed at that time.
    pub st_sol_supply: StLamports,

    /// The amount of SOL we managed at that time, according to our internal
    /// bookkeeping, so excluding the validation rewards paid at the start of
    /// epoch `computed_in_epoch`.
    pub sol_balance: Lamports,
}

impl ExchangeRate {
    /// Convert SOL to stSOL.
    pub fn exchange_sol(&self, amount: Lamports) -> token::Result<StLamports> {
        // The exchange rate starts out at 1:1, if there are no deposits yet.
        // If we minted stSOL but there is no SOL, then also assume a 1:1 rate.
        if self.st_sol_supply == StLamports(0) || self.sol_balance == Lamports(0) {
            return Ok(StLamports(amount.0));
        }

        let rate = Rational {
            numerator: self.st_sol_supply.0,
            denominator: self.sol_balance.0,
        };

        // The result is in Lamports, because the type system considers Rational
        // dimensionless, but in this case `rate` has dimensions stSOL/SOL, so
        // we need to re-wrap the result in the right type.
        (amount * rate).map(|x| StLamports(x.0))
    }

    /// Convert stSOL to SOL.
    pub fn exchange_st_sol(&self, amount: StLamports) -> std::result::Result<Lamports, LidoError> {
        // If there is no stSOL in existence, it cannot be exchanged.
        if self.st_sol_supply == StLamports(0) {
            return Err(LidoError::InvalidAmount);
        }

        let rate = Rational {
            numerator: self.sol_balance.0,
            denominator: self.st_sol_supply.0,
        };

        // The result is in StLamports, because the type system considers Rational
        // dimensionless, but in this case `rate` has dimensions SOL/stSOL, so
        // we need to re-wrap the result in the right type.
        Ok((amount * rate).map(|x| Lamports(x.0))?)
    }
}

#[account]
#[derive(
Debug, Default, Eq, PartialEq,
)]
pub struct Lido {
    /// Version number for the Lido
    pub lido_version: u8,

    /// Manager of the Lido program, able to execute administrative functions
    pub manager: Pubkey,

    /// The SPL Token mint address for stSOL.
    pub st_sol_mint: Pubkey,

    /// Exchange rate to use when depositing.
    pub exchange_rate: ExchangeRate,

    /// Bump seeds for signing messages on behalf of the authority
    pub sol_reserve_account_bump_seed: u8,
    pub stake_authority_bump_seed: u8,
    pub mint_authority_bump_seed: u8,
    pub rewards_withdraw_authority_bump_seed: u8,

    /// How rewards are distributed.
    pub reward_distribution: RewardDistribution,

    /// Accounts of the fee recipients.
    pub fee_recipients: FeeRecipients,

    /// Metrics for informational purposes.
    ///
    /// Metrics are only written to, no program logic should depend on these values.
    /// An off-chain program can load a snapshot of the `Lido` struct, and expose
    /// these metrics.
    pub metrics: Metrics,

    /// Map of enrolled validators, maps their vote account to `Validator` details.
    pub validators: Validators,

    /// The set of maintainers.
    ///
    /// Maintainers are granted low security risk privileges. Maintainers are
    /// expected to run the maintenance daemon, that invokes the maintenance
    /// operations. These are gated on the signer being present in this set.
    /// In the future we plan to make maintenance operations callable by anybody.
    pub maintainers: Maintainers,
}


// impl Lido

#[derive(Clone, Debug, Eq, PartialEq, AnchorDeserialize, AnchorSerialize)]
pub struct Validator {
    /// Fees in stSOL that the validator is entitled too, but hasn't claimed yet.
    pub fee_credit: StLamports,

    /// SPL token account denominated in stSOL to transfer fees to when claiming them.
    pub fee_address: Pubkey,

    /// Seeds for active stake accounts.
    pub stake_seeds: SeedRange,
    /// Seeds for inactive stake accounts.
    pub unstake_seeds: SeedRange,

    /// Sum of the balances of the stake accounts and unstake accounts.
    pub stake_accounts_balance: Lamports,

    /// Sum of the balances of the unstake accounts.
    pub unstake_accounts_balance: Lamports,

    /// Controls if a validator is allowed to have new stake deposits.
    /// When removing a validator, this flag should be set to `false`.
    pub active: bool,
}

#[derive(
Clone, Debug, Default, Eq, PartialEq, AnchorDeserialize, AnchorSerialize,
)]
pub struct SeedRange {
    /// Start (inclusive) of the seed range for stake accounts.
    ///
    /// When we stake deposited SOL, we take it out of the reserve account, and
    /// transfer it to a stake account. The stake account address is a derived
    /// address derived from a.o. the validator address, and a seed. After
    /// creation, it takes one or more epochs for the stake to become fully
    /// activated. While stake is activating, we may want to activate additional
    /// stake, so we need a new stake account. Therefore we have a range of
    /// seeds. When we need a new stake account, we bump `end`. When the account
    /// with seed `begin` is 100% active, we deposit that stake account into the
    /// pool and bump `begin`. Accounts are not reused.
    ///
    /// The program enforces that creating new stake accounts is only allowed at
    /// the `end` seed, and depositing active stake is only allowed from the
    /// `begin` seed. This ensures that maintainers don’t race and accidentally
    /// stake more to this validator than intended. If the seed has changed
    /// since the instruction was created, the transaction fails.
    ///
    /// When we unstake SOL, we follow an analogous symmetric mechanism. We
    /// split the validator's stake in two, and retrieve the funds of the second
    /// to the reserve account where it can be re-staked.
    pub begin: u64,

    /// End (exclusive) of the seed range for stake accounts.
    pub end: u64,
}

impl IntoIterator for &SeedRange {
    type Item = u64;
    type IntoIter = Range<u64>;

    fn into_iter(self) -> Self::IntoIter {
        Range {
            start: self.begin,
            end: self.end,
        }
    }
}

impl Validator {
    pub fn new(fee_address: Pubkey) -> Validator {
        Validator {
            fee_address,
            ..Default::default()
        }
    }

    /// Return the balance in only the stake accounts, excluding the unstake accounts.
    pub fn effective_stake_balance(&self) -> Lamports {
        (self.stake_accounts_balance - self.unstake_accounts_balance)
            .expect("Unstake balance cannot exceed the validator's total stake balance.")
    }
}

impl Default for Validator {
    fn default() -> Self {
        Validator {
            fee_address: Pubkey::default(),
            fee_credit: StLamports(0),
            stake_seeds: SeedRange { begin: 0, end: 0 },
            unstake_seeds: SeedRange { begin: 0, end: 0 },
            stake_accounts_balance: Lamports(0),
            unstake_accounts_balance: Lamports(0),
            active: true,
        }
    }
}

// impl Validator

// impl PubkeyAndEntry<Validator> {

/// Determines how rewards are split up among these parties, represented as the
/// number of parts of the total. For example, if each party has 1 part, then
/// they all get an equal share of the reward.
#[derive(
Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize,
)]
pub struct RewardDistribution {
    pub treasury_fee: u32,
    pub validation_fee: u32,
    pub developer_fee: u32,
    pub st_sol_appreciation: u32,
}

/// Specifies the fee recipients, accounts that should be created by Lido's minter
#[derive(
Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize,
)]
pub struct FeeRecipients {
    pub treasury_account: Pubkey,
    pub developer_account: Pubkey,
}

// impl RewardDistribution

/// The result of [`RewardDistribution::split_reward`].
///
/// It contains only the fees. The amount that goes to stSOL value appreciation
/// is implicitly the remainder.
#[derive(Debug, PartialEq, Eq)]
pub struct Fees {
    pub treasury_amount: Lamports,
    pub reward_per_validator: Lamports,
    pub developer_amount: Lamports,

    /// Remainder of the reward.
    ///
    /// This is not a fee, and it is not paid out explicitly, but when summed
    /// with the other fields in this struct, that totals the input amount.
    pub st_sol_appreciation_amount: Lamports,
}

#[account]
#[derive(Default)]
pub struct Reserve {}