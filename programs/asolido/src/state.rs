// SPDX-FileCopyrightText: 2021 Chorus One AG
// SPDX-License-Identifier: GPL-3.0

//! State transition types

use crate::error::LidoError;
use crate::metrics::Metrics;
use crate::token;
use crate::token::{Lamports, Rational, StLamports};
use anchor_lang::prelude::*;
use std::ops::Range;
use crate::validators::{Validators, PubkeyAndEntry};
use crate::maintainers::Maintainers;

pub const LIDO_VERSION: u8 = 0;

/// Size of a serialized `Lido` struct excluding validators and maintainers.
pub const LIDO_CONSTANT_SIZE: usize = 357;

pub const VALIDATOR_CONSTANT_SIZE: usize = 89;

impl Validators {
    pub fn iter_active(&self) -> impl Iterator<Item = &Validator> {
        self.iter_entries().filter(|&v| v.active)
    }

    pub fn iter_active_entries(&self) -> impl Iterator<Item = &PubkeyAndEntry> {
        self.entries.iter().filter(|&v| v.entry.active)
    }
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
#[derive(Clone, Debug, Default, AnchorDeserialize, AnchorSerialize, Eq, PartialEq)]
pub struct ExchangeRate {
    /// The epoch in which we last called `UpdateExchangeRate`.
    pub computed_in_epoch: u64,

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
        if self.st_sol_supply == StLamports::new(0) || self.sol_balance == Lamports::new(0) {
            return Ok(StLamports::new(amount.amount));
        }

        let rate = Rational {
            numerator: self.st_sol_supply.amount,
            denominator: self.sol_balance.amount,
        };

        // The result is in Lamports, because the type system considers Rational
        // dimensionless, but in this case `rate` has dimensions stSOL/SOL, so
        // we need to re-wrap the result in the right type.
        (amount * rate).map(|x| StLamports::new(x.amount))
    }

    /// Convert stSOL to SOL.
    pub fn exchange_st_sol(&self, amount: StLamports) -> std::result::Result<Lamports, LidoError> {
        // If there is no stSOL in existence, it cannot be exchanged.
        if self.st_sol_supply == StLamports::new(0) {
            return Err(LidoError::InvalidAmount);
        }

        let rate = Rational {
            numerator: self.sol_balance.amount,
            denominator: self.st_sol_supply.amount,
        };

        // The result is in StLamports, because the type system considers Rational
        // dimensionless, but in this case `rate` has dimensions SOL/stSOL, so
        // we need to re-wrap the result in the right type.
        Ok((amount * rate).map(|x| Lamports::new(x.amount))?)
    }
}

#[account]
#[derive(Debug, Default, Eq, PartialEq)]
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

#[derive(Clone, Debug, Default, Eq, PartialEq, AnchorDeserialize, AnchorSerialize)]
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
            fee_credit: StLamports::new(0),
            stake_seeds: SeedRange { begin: 0, end: 0 },
            unstake_seeds: SeedRange { begin: 0, end: 0 },
            stake_accounts_balance: Lamports::new(0),
            unstake_accounts_balance: Lamports::new(0),
            active: true,
        }
    }
}

// impl Validator

// impl PubkeyAndEntry<Validator> {

/// Determines how rewards are split up among these parties, represented as the
/// number of parts of the total. For example, if each party has 1 part, then
/// they all get an equal share of the reward.
#[derive(Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct RewardDistribution {
    pub treasury_fee: u32,
    pub validation_fee: u32,
    pub developer_fee: u32,
    pub st_sol_appreciation: u32,
}

/// Specifies the fee recipients, accounts that should be created by Lido's minter
#[derive(Clone, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize)]
pub struct FeeRecipients {
    pub treasury_account: Pubkey,
    pub developer_account: Pubkey,
}

impl RewardDistribution {
    pub fn sum(&self) -> u64 {
        // These adds don't overflow because we widen from u32 to u64 first.
        self.treasury_fee as u64
            + self.validation_fee as u64
            + self.developer_fee as u64
            + self.st_sol_appreciation as u64
    }

    pub fn treasury_fraction(&self) -> Rational {
        Rational {
            numerator: self.treasury_fee as u64,
            denominator: self.sum(),
        }
    }

    pub fn validation_fraction(&self) -> Rational {
        Rational {
            numerator: self.validation_fee as u64,
            denominator: self.sum(),
        }
    }

    pub fn developer_fraction(&self) -> Rational {
        Rational {
            numerator: self.developer_fee as u64,
            denominator: self.sum(),
        }
    }

    /// Split the reward according to the distribution defined in this instance.
    ///
    /// Fees are all rounded down, and the remainder goes to stSOL appreciation.
    /// This means that the outputs may not sum to the input, even when
    /// `st_sol_appreciation` is 0.
    ///
    /// Returns the fee amounts in SOL. stSOL should be minted for those when
    /// they get distributed. This acts like a deposit: it is like the fee
    /// recipients received their fee in SOL outside of Solido, and then
    /// deposited it. The remaining SOL, which is not taken as a fee, acts as a
    /// donation to the pool, and makes the SOL value of stSOL go up. It is not
    /// included in the output, as nothing needs to be done to handle it.
    pub fn split_reward(&self, amount: Lamports, num_validators: u64) -> token::Result<Fees> {
        use std::ops::Add;

        let treasury_amount = (amount * self.treasury_fraction())?;
        let developer_amount = (amount * self.developer_fraction())?;

        // The actual amount that goes to validation can be a tiny bit lower
        // than the target amount, when the number of validators does not divide
        // the target amount. The loss is at most `num_validators` Lamports.
        let validation_amount = (amount * self.validation_fraction())?;
        let reward_per_validator = (validation_amount / num_validators)?;

        // Sanity check: We should not produce more fees than we had to split in
        // the first place.
        let total_fees = Lamports::new(0)
            .add(treasury_amount)?
            .add(developer_amount)?
            .add((reward_per_validator * num_validators)?)?;
        assert!(total_fees <= amount);

        let st_sol_appreciation_amount = (amount - total_fees)?;

        let result = Fees {
            treasury_amount,
            reward_per_validator,
            developer_amount,
            st_sol_appreciation_amount,
        };

        Ok(result)
    }
}

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

#[cfg(test)]
mod test_lido {
    use super::Fees;
    use super::*;

    #[test]
    fn test_account_map_required_bytes_relates_to_maximum_entries() {
        for buffer_size in 0..8_000 {
            let max_entries = Validators::maximum_entries(buffer_size);
            let needed_size = Validators::required_bytes(max_entries);
            assert!(
                needed_size <= buffer_size || max_entries == 0,
                "Buffer of len {} can fit {} validators which need {} bytes.",
                buffer_size,
                max_entries,
                needed_size,
            );

            let max_entries = Maintainers::maximum_entries(buffer_size);
            let needed_size = Maintainers::required_bytes(max_entries);
            assert!(
                needed_size <= buffer_size || max_entries == 0,
                "Buffer of len {} can fit {} maintainers which need {} bytes.",
                buffer_size,
                max_entries,
                needed_size,
            );
        }
    }

    #[test]
    fn test_exchange_when_balance_and_supply_are_zero() {
        let rate = ExchangeRate {
            computed_in_epoch: 0,
            sol_balance: Lamports::new(0),
            st_sol_supply: StLamports::new(0),
        };
        assert_eq!(
            rate.exchange_sol(Lamports::new(123)),
            Ok(StLamports::new(123))
        );
    }

    #[test]
    fn test_exchange_when_rate_is_one_to_two() {
        let rate = ExchangeRate {
            computed_in_epoch: 0,
            sol_balance: Lamports::new(2),
            st_sol_supply: StLamports::new(1),
        };
        // If every stSOL is worth 1 SOL, I should get half my SOL amount in stSOL.
        assert_eq!(
            rate.exchange_sol(Lamports::new(44)),
            Ok(StLamports::new(22))
        );
    }

    #[test]
    fn test_exchange_when_one_balance_is_zero() {
        // This case can occur when we donate some SOL to Lido, instead of
        // depositing it. There will not be any stSOL, but there will be SOL.
        // In this case it doesn't matter which exchange rate we use, the first
        // deposits will mint some stSOL, and that stSOL will own all of the
        // pool. The rate we choose is only nominal, it controls the initial
        // stSOL:SOL rate, and we choose it to be 1:1.
        let rate = ExchangeRate {
            computed_in_epoch: 0,
            sol_balance: Lamports::new(100),
            st_sol_supply: StLamports::new(0),
        };
        assert_eq!(
            rate.exchange_sol(Lamports::new(123)),
            Ok(StLamports::new(123))
        );

        // This case should not occur in the wild, but in any case, use a 1:1 rate here too.
        let rate = ExchangeRate {
            computed_in_epoch: 0,
            sol_balance: Lamports::new(0),
            st_sol_supply: StLamports::new(100),
        };
        assert_eq!(
            rate.exchange_sol(Lamports::new(123)),
            Ok(StLamports::new(123))
        );
    }

    #[test]
    fn test_exchange_sol_to_st_sol_to_sol_roundtrips() {
        // There are many cases where depositing some amount of SOL and then
        // exchanging it back, does not actually roundtrip. There can be small
        // losses due to integer arithmetic rounding, but there can even be large
        // losses, if the sol_balance and st_sol_supply are very different. For
        // example, if sol_balance = 10, st_sol_supply = 1, then if you deposit
        // 9 Lamports, you are entitled to 0.1 stLamports, which gets rounded
        // down to 0, and you lose your full 9 Lamports.
        // So here we test a few of those cases as a sanity check, but it's not
        // a general roundtripping test.
        let rate = ExchangeRate {
            computed_in_epoch: 0,
            sol_balance: Lamports::new(100),
            st_sol_supply: StLamports::new(50),
        };
        let sol_1 = Lamports::new(10);
        let st_sol = rate.exchange_sol(sol_1).unwrap();
        let sol_2 = rate.exchange_st_sol(st_sol).unwrap();
        assert_eq!(sol_2, sol_1);

        // In this case, one Lamport is lost in a rounding error, because
        // `amount * st_sol_supply` is not a multiple of `sol_balance`.
        let rate = ExchangeRate {
            computed_in_epoch: 0,
            sol_balance: Lamports::new(110_000),
            st_sol_supply: StLamports::new(100_000),
        };
        let sol_1 = Lamports::new(1_000);
        let st_sol = rate.exchange_sol(sol_1).unwrap();
        let sol_2 = rate.exchange_st_sol(st_sol).unwrap();
        assert_eq!(sol_2, Lamports::new(999));
    }

    /*
    #[test]
    fn test_lido_for_deposit_wrong_mint() {
        let mut lido = Lido::default();
        lido.st_sol_mint = Pubkey::new_unique();

        let pubkey = Pubkey::new_unique();
        let mut lamports = 100;
        let mut data = [0_u8];
        let is_signer = false;
        let is_writable = false;
        let owner = spl_token::id();
        let executable = false;
        let rent_epoch = 1;
        let fake_mint_account = AccountInfo::new(
            &pubkey,
            is_signer,
            is_writable,
            &mut lamports,
            &mut data,
            &owner,
            executable,
            rent_epoch,
        );
        let result = lido.check_mint_is_st_sol_mint(&fake_mint_account);

        let expected_error: ProgramError = LidoError::InvalidStSolAccount.into();
        assert_eq!(result, Err(expected_error));
    }

    #[test]
    fn test_get_sol_balance() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let rent = &Rent::default();
        let mut lido = Lido::default();
        let key = Pubkey::default();
        let mut amount = rent.minimum_balance(0);
        let mut reserve_account =
            AccountInfo::new(&key, true, true, &mut amount, &mut [], &key, false, 0);

        assert_eq!(
            lido.get_sol_balance(&rent, &reserve_account),
            Ok(Lamports::new(0))
        );

        let mut new_amount = rent.minimum_balance(0) + 10;
        reserve_account.lamports = Rc::new(RefCell::new(&mut new_amount));

        assert_eq!(
            lido.get_sol_balance(&rent, &reserve_account),
            Ok(Lamports(10))
        );

        lido.validators.maximum_entries = 1;
        lido.validators
            .add(Pubkey::new_unique(), Validator::new(Pubkey::new_unique()))
            .unwrap();
        lido.validators.entries[0].entry.stake_accounts_balance = Lamports(37);
        assert_eq!(
            lido.get_sol_balance(&rent, &reserve_account),
            Ok(Lamports(10 + 37))
        );

        lido.validators.entries[0].entry.stake_accounts_balance = Lamports(u64::MAX);

        assert_eq!(
            lido.get_sol_balance(&rent, &reserve_account),
            Err(LidoError::CalculationFailure)
        );

        let mut new_amount = u64::MAX;
        reserve_account.lamports = Rc::new(RefCell::new(&mut new_amount));
        // The amount here is more than the rent exemption that gets discounted
        // from the reserve, causing an overflow.
        lido.validators.entries[0].entry.stake_accounts_balance = Lamports(5_000_000);

        assert_eq!(
            lido.get_sol_balance(&rent, &reserve_account),
            Err(LidoError::CalculationFailure)
        );
    }

    #[test]
    fn test_get_st_sol_supply() {
        use solana_program::program_option::COption;

        let mint = Mint {
            mint_authority: COption::None,
            supply: 200_000,
            decimals: 9,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut data = [0_u8; 128];
        mint.pack_into_slice(&mut data);

        let mut lido = Lido::default();
        let mint_address = Pubkey::default();
        let mut amount = 0;
        let is_signer = false;
        let is_writable = false;
        let executable = false;
        let rent_epoch = 0;
        let st_sol_mint = AccountInfo::new(
            &mint_address,
            is_signer,
            is_writable,
            &mut amount,
            &mut data,
            &mint_address,
            executable,
            rent_epoch,
        );

        lido.st_sol_mint = mint_address;

        assert_eq!(
            lido.get_st_sol_supply(&st_sol_mint),
            Ok(StLamports(200_000)),
        );

        lido.validators.maximum_entries = 1;
        lido.validators
            .add(Pubkey::new_unique(), Validator::new(Pubkey::new_unique()))
            .unwrap();
        lido.validators.entries[0].entry.fee_credit = StLamports(37);
        assert_eq!(
            lido.get_st_sol_supply(&st_sol_mint),
            Ok(StLamports(200_000 + 37))
        );

        lido.st_sol_mint = Pubkey::new_unique();

        assert_eq!(
            lido.get_st_sol_supply(&st_sol_mint),
            Err(LidoError::InvalidStSolAccount.into())
        );
    } */

    #[test]
    fn test_split_reward() {
        let mut spec = RewardDistribution {
            treasury_fee: 3,
            validation_fee: 2,
            developer_fee: 1,
            st_sol_appreciation: 0,
        };

        assert_eq!(
            // In this case the amount can be split exactly,
            // there is no remainder.
            spec.split_reward(Lamports::new(600), 1).unwrap(),
            Fees {
                treasury_amount: Lamports::new(300),
                reward_per_validator: Lamports::new(200),
                developer_amount: Lamports::new(100),
                st_sol_appreciation_amount: Lamports::new(0),
            },
        );

        assert_eq!(
            // In this case the amount cannot be split exactly, all fees are
            // rounded down.
            spec.split_reward(Lamports::new(1_000), 4).unwrap(),
            Fees {
                treasury_amount: Lamports::new(500),
                reward_per_validator: Lamports::new(83),
                developer_amount: Lamports::new(166),
                st_sol_appreciation_amount: Lamports::new(2),
            },
        );

        // If we use 3%, 2%, 1% fee, and the remaining 94% go to stSOL appreciation,
        // we should see 3%, 2%, and 1% fee.
        spec.st_sol_appreciation = 94;
        assert_eq!(
            spec.split_reward(Lamports::new(100), 1).unwrap(),
            Fees {
                treasury_amount: Lamports::new(3),
                reward_per_validator: Lamports::new(2),
                developer_amount: Lamports::new(1),
                st_sol_appreciation_amount: Lamports::new(94),
            },
        );

        let spec_coprime = RewardDistribution {
            treasury_fee: 17,
            validation_fee: 23,
            developer_fee: 19,
            st_sol_appreciation: 0,
        };
        assert_eq!(
            spec_coprime.split_reward(Lamports::new(1_000), 1).unwrap(),
            Fees {
                treasury_amount: Lamports::new(288),
                reward_per_validator: Lamports::new(389),
                developer_amount: Lamports::new(322),
                st_sol_appreciation_amount: Lamports::new(1),
            },
        );
    }
}
