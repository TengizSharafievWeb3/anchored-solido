use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Mint};
use solana_program::program_option::COption;
use crate::state::{Lido, Reserve};
use crate::token::{Lamports, StLamports};
use crate::state::{LIDO_VERSION, RewardDistribution};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod initialize;
pub mod state;
pub mod logic;
pub mod error;
mod account_map;
mod token;


#[program]
pub mod asolido {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        reward_distribution: RewardDistribution,
        max_validators: u32,
        max_maintainers: u32,
    ) -> Result<()> {
        ctx.accounts.process(LIDO_VERSION, reward_distribution, max_validators, max_maintainers)
    }

    /// Deposit a given amount of SOL.
    ///
    /// This can be called by anybody.
    pub fn deposit(ctx: Context<Deposit>, amount: Lamports) -> Result<()> {
        Ok(())
    }

    /// Withdraw a given amount of stSOL.
    ///
    /// Caller provides some `amount` of StLamports that are to be burned in
    /// order to withdraw SOL.
    pub fn withdraw(ctx: Context<Withdraw>, amount: StLamports) -> Result<()> {
        Ok(())
    }

    /// Move deposits from the reserve into a stake account and delegate it to a member validator.
    pub fn stake_deposit(ctx: Context<StakeDeposit>, amount: Lamports) -> Result<()> {
        Ok(())
    }

    /// Unstake from a validator to a new stake account.
    pub fn unstake(ctx: Context<Unstake>, amount: Lamports) -> Result<()> {
        Ok(())
    }

    /// Update the exchange rate, at the beginning of the epoch.
    pub fn update_exchange_rate(ctx: Context<UpdateExchangeRate>) -> Result<()> {
        Ok(())
    }

    /// Observe any external changes in the balances of a validator's stake accounts.
    ///
    /// If there is inactive balance in stake accounts, withdraw this back to the reserve.
    pub fn withdraw_inactive_stake(ctx: Context<WithdrawInactiveStake>) -> Result<()> {
        Ok(())
    }

    pub fn collect_validator_fee(ctx: Context<CollectValidatorFee>) -> Result<()> {
        Ok(())
    }

    pub fn claim_validator_fee(ctx: Context<ClaimValidatorFee>) -> Result<()> {
        Ok(())
    }

    pub fn change_reward_distribution(
        ctx: Context<ChangeRewardDistribution>,
        new_reward_distribution: RewardDistribution,
    ) -> Result<()> {
        Ok(())
    }

    /// Add a new validator to the validator set.
    pub fn add_validator(ctx: Context<AddValidator>) -> Result<()> {
        Ok(())
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
    pub fn deactivate_validator(ctx: Context<DeactivateValidator>) -> Result<()> {
        Ok(())
    }

    pub fn remove_validator(ctx: Context<RemoveValidator>) -> Result<()> {
        Ok(())
    }

    pub fn add_maintainer(ctx: Context<AddMaintainer>) -> Result<()> {
        Ok(())
    }

    pub fn remove_maintainer(ctx: Context<RemoveMaintainer>) -> Result<()> {
        Ok(())
    }

    pub fn merge_stake(ctx: Context<MergeStake>) -> Result<()> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------

/// Seed for reserve account that holds SOL.
pub const RESERVE_ACCOUNT: &[u8] = b"reserve_account";

/// Mint authority, mints StSol.
pub const MINT_AUTHORITY: &[u8] = b"mint_authority";

/// Seed for managing the stake.
pub const STAKE_AUTHORITY: &[u8] = b"stake_authority";

/// Additional seed for active/activating validator stake accounts.
pub const VALIDATOR_STAKE_ACCOUNT: &[u8] = b"validator_stake_account";
/// Additional seed for inactive/deactivating validator stake accounts.
pub const VALIDATOR_UNSTAKE_ACCOUNT: &[u8] = b"validator_unstake_account";

/// Authority responsible for withdrawing the stake rewards.
pub const REWARDS_WITHDRAW_AUTHORITY: &[u8] = b"rewards_withdraw_authority";

// ----------------------------------------------------------------------------

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer)]
    pub lido: Account<'info, Lido>,

    pub manager: UncheckedAccount<'info>,
    #[account(rent_exempt = enforce)]
    pub st_sol_mint: Account<'info, Mint>,
    #[account(constraint = treasury_account.mint == st_sol_mint.key() @ error::LidoError::InvalidFeeRecipient)]
    pub treasury_account: Account<'info, TokenAccount>,
    #[account(constraint = developer_account.mint == st_sol_mint.key() @ error::LidoError::InvalidFeeRecipient)]
    pub developer_account: Account<'info, TokenAccount>,
    #[account(rent_exempt = enforce, seeds = [RESERVE_ACCOUNT.as_ref()], bump)]
    pub reserve_account: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
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
pub struct AddValidator {}

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