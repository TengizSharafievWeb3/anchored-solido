use crate::error::LidoError;
use crate::state::Lido;
use crate::state::{RewardDistribution, LIDO_VERSION};
use crate::token::{Lamports, StLamports};
use crate::vote_state::PartialVoteState;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};
use solana_program::program_option::COption;

declare_id!("BjYuhzR84Wovp7KVtTcej6Rr5X1KsnDdG4qDXz8KZk3M");

pub mod error;
pub mod initialize;
pub mod logic;
pub mod maintainers;
pub mod metrics;
pub mod process_validator;
pub mod state;
pub mod token;
pub mod validators;
pub mod vote_state;

#[program]
pub mod asolido {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        reward_distribution: RewardDistribution,
        max_validators: u32,
        max_maintainers: u32,
    ) -> Result<()> {
        ctx.accounts.process(
            &ctx.bumps,
            LIDO_VERSION,
            reward_distribution,
            max_validators,
            max_maintainers,
        )
    }

    /// Deposit a given amount of SOL.
    ///
    /// This can be called by anybody.
    #[allow(unused_variables)]
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        todo!()
    }

    /// Withdraw a given amount of stSOL.
    ///
    /// Caller provides some `amount` of StLamports that are to be burned in
    /// order to withdraw SOL.
    #[allow(unused_variables)]
    pub fn withdraw(ctx: Context<Withdraw>, amount: StLamports) -> Result<()> {
        todo!()
    }

    /// Move deposits from the reserve into a stake account and delegate it to a member validator.
    #[allow(unused_variables)]
    pub fn stake_deposit(ctx: Context<StakeDeposit>, amount: Lamports) -> Result<()> {
        todo!()
    }

    /// Unstake from a validator to a new stake account.
    #[allow(unused_variables)]
    pub fn unstake(ctx: Context<Unstake>, amount: Lamports) -> Result<()> {
        todo!()
    }

    /// Update the exchange rate, at the beginning of the epoch.
    #[allow(unused_variables)]
    pub fn update_exchange_rate(ctx: Context<UpdateExchangeRate>) -> Result<()> {
        todo!()
    }

    /// Observe any external changes in the balances of a validator's stake accounts.
    ///
    /// If there is inactive balance in stake accounts, withdraw this back to the reserve.
    #[allow(unused_variables)]
    pub fn withdraw_inactive_stake(ctx: Context<WithdrawInactiveStake>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn collect_validator_fee(ctx: Context<CollectValidatorFee>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn claim_validator_fee(ctx: Context<ClaimValidatorFee>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn change_reward_distribution(
        ctx: Context<ChangeRewardDistribution>,
        new_reward_distribution: RewardDistribution,
    ) -> Result<()> {
        Ok(())
    }

    /// Add a new validator to the validator set.
    pub fn add_validator(ctx: Context<AddValidator>) -> Result<()> {
        ctx.accounts.process()
    }

    /// Set the `active` flag to false for a given validator.
    ///
    /// Requires the manager to sign.
    ///
    /// Deactivation initiates the validator removal process:
    ///
    /// * It prevents new funds from being staked with the validator.
    /// * It signals to the maintainer bot to start unstaking from this validator.
    ///
    /// Once there are no more delegations to this validator, and it has no
    /// unclaimed fee credits, then the validator can be removed.
    #[allow(unused_variables)]
    pub fn deactivate_validator(ctx: Context<DeactivateValidator>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn remove_validator(ctx: Context<RemoveValidator>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn add_maintainer(ctx: Context<AddMaintainer>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn remove_maintainer(ctx: Context<RemoveMaintainer>) -> Result<()> {
        todo!()
    }

    #[allow(unused_variables)]
    pub fn merge_stake(ctx: Context<MergeStake>) -> Result<()> {
        todo!()
    }
}

// ----------------------------------------------------------------------------

/// Seed for reserve account that holds SOL.
pub const RESERVE_ACCOUNT: [u8; 15] = *b"reserve_account";

/// Mint authority, mints StSol.
pub const MINT_AUTHORITY: [u8; 14] = *b"mint_authority";

/// Seed for managing the stake.
pub const STAKE_AUTHORITY: [u8; 15] = *b"stake_authority";

/// Additional seed for active/activating validator stake accounts.
pub const VALIDATOR_STAKE_ACCOUNT: [u8; 23] = *b"validator_stake_account";
/// Additional seed for inactive/deactivating validator stake accounts.
pub const VALIDATOR_UNSTAKE_ACCOUNT: [u8; 25] = *b"validator_unstake_account";

/// Authority responsible for withdrawing the stake rewards.
pub const REWARDS_WITHDRAW_AUTHORITY: [u8; 26] = *b"rewards_withdraw_authority";

// ----------------------------------------------------------------------------

#[derive(Accounts)]
#[instruction(max_validators: u32, max_maintainers: u32)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = Initialize::required_bytes(max_validators, max_maintainers))]
    pub lido: Box<Account<'info, Lido>>,

    /// CHECK: This is not dangerous because we don't read or write from this account
    pub manager: UncheckedAccount<'info>,

    /// Check if the mint program coin supply is zero and the mint authority is set
    /// to `mint_authority`.
    #[account(
        rent_exempt = enforce,
        constraint = st_sol_mint.supply == 0 @ LidoError::InvalidMint,
        constraint = st_sol_mint.mint_authority == COption::Some(mint_authority.key()) @ LidoError::InvalidMint,
    )]
    pub st_sol_mint: Account<'info, Mint>,

    #[account(constraint = treasury.mint == st_sol_mint.key() @ LidoError::InvalidFeeRecipient)]
    pub treasury: Account<'info, TokenAccount>,
    #[account(constraint = developer.mint == st_sol_mint.key() @ LidoError::InvalidFeeRecipient)]
    pub developer: Account<'info, TokenAccount>,

    #[account(rent_exempt = enforce, seeds = [lido.key().as_ref(), RESERVE_ACCOUNT.as_ref()], bump)]
    /// CHECK: Checked above, used only for bump calc and rent_exempt check
    pub reserve: UncheckedAccount<'info>,

    #[account(seeds = [lido.key().as_ref(), MINT_AUTHORITY.as_ref()], bump)]
    /// CHECK: Checked above, used only for bump calc
    pub mint_authority: UncheckedAccount<'info>,

    #[account(seeds = [lido.key().as_ref(), STAKE_AUTHORITY.as_ref()], bump)]
    /// CHECK: Checked above, used only for bump calc
    pub stake_authority: UncheckedAccount<'info>,

    #[account(seeds = [lido.key().as_ref(), REWARDS_WITHDRAW_AUTHORITY.as_ref()], bump)]
    /// CHECK: Checked above, used only for bump calc
    pub rewards_withdraw_authority: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// ----------------------------------------------------------------------------

#[derive(Accounts)]
pub struct Deposit {}

#[derive(Accounts)]
pub struct Withdraw {}

#[derive(Accounts)]
pub struct StakeDeposit {}

#[derive(Accounts)]
pub struct Unstake {}

#[derive(Accounts)]
pub struct UpdateExchangeRate {}

#[derive(Accounts)]
pub struct WithdrawInactiveStake {}

#[derive(Accounts)]
pub struct CollectValidatorFee {}

#[derive(Accounts)]
pub struct ClaimValidatorFee {}

#[derive(Accounts)]
pub struct ChangeRewardDistribution {}

#[derive(Accounts)]
pub struct AddValidator<'info> {
    #[account(mut, has_one = manager @ LidoError::InvalidManager)]
    pub lido: Box<Account<'info, Lido>>,

    pub manager: Signer<'info>,

    #[account(
        rent_exempt = enforce,
        constraint = validator_vote.version == 1 @ LidoError::InvalidVoteAccount,
        constraint = validator_vote.authorized_withdrawer == rewards_withdraw_authority.key() @ LidoError::InvalidVoteAccount,
        constraint = validator_vote.commission == 100 @ LidoError::InvalidVoteAccount,
    )]
    pub validator_vote: Account<'info, PartialVoteState>,

    #[account(constraint = validator_fee_st_sol.mint == lido.st_sol_mint @ LidoError::InvalidFeeRecipient)]
    pub validator_fee_st_sol: Account<'info, TokenAccount>,

    #[account(seeds = [lido.key().as_ref(), REWARDS_WITHDRAW_AUTHORITY.as_ref()], bump)]
    /// CHECK: Checked above, used only for bump calc
    pub rewards_withdraw_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct DeactivateValidator {}

#[derive(Accounts)]
pub struct RemoveValidator {}

#[derive(Accounts)]
pub struct AddMaintainer {}

#[derive(Accounts)]
pub struct RemoveMaintainer {}

#[derive(Accounts)]
pub struct MergeStake {}
