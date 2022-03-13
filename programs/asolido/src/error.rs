use anchor_lang::prelude::*;
use num_derive::FromPrimitive;
use crate::token::ArithmeticError;

#[error_code]
#[derive(Eq, FromPrimitive, PartialEq)]
pub enum LidoError {
    /// Address is already initialized
    AlreadyInUse,

    /// Lido account mismatch the one stored in the Lido program
    InvalidOwner,

    /// Invalid allocated amount
    InvalidAmount,

    /// A required signature is missing
    SignatureMissing,

    /// The reserve account is invalid
    InvalidReserveAccount,

    /// Calculation failed due to division by zero or overflow
    CalculationFailure,

    /// Stake account does not exist or is in an invalid state
    WrongStakeState,

    /// The sum of numerators should be equal to the denominators
    InvalidFeeAmount,

    /// Number of maximum validators reached
    MaximumNumberOfAccountsExceeded,

    /// The size of the account for the Solido state does not match `max_validators`.
    UnexpectedMaxValidators,

    /// Wrong manager trying  to alter the state
    InvalidManager,

    /// Wrong maintainer trying  to alter the state
    InvalidMaintainer,

    /// One of the provided accounts had a mismatch in is_writable or is_signer,
    /// or for a const account, the address does not match the expected address.
    InvalidAccountInfo,

    /// More accounts were provided than the program expects.
    TooManyAccountKeys,

    /// Wrong fee distribution account
    InvalidFeeDistributionAccount,

    /// Wrong validator credits account
    InvalidValidatorCreditAccount,

    /// Validator credit account was changed
    ValidatorCreditChanged,

    /// Fee account should be the same as the Stake pool fee'
    InvalidFeeAccount,

    /// One of the fee recipients is invalid
    InvalidFeeRecipient,

    /// There is a stake account with the same key present in the validator
    /// credit list.
    DuplicatedEntry,

    /// Validator credit account was not found
    ValidatorCreditNotFound,

    /// Validator has unclaimed credit, should mint the tokens before the validator removal
    ValidatorHasUnclaimedCredit,

    /// The reserve account is not rent exempt
    ReserveIsNotRentExempt,

    /// The requested amount for reserve withdrawal exceeds the maximum held in
    /// the reserve account considering rent exemption
    AmountExceedsReserve,

    /// The same maintainer's public key already exists in the structure
    DuplicatedMaintainer,

    /// A member of the accounts list (maintainers or validators) is not present
    /// in the structure
    InvalidAccountMember,

    /// Lido has an invalid size, calculated with the Lido's constant size plus
    /// required to hold variable structures
    InvalidLidoSize,

    /// The instance has no validators.
    NoActiveValidators,

    /// When staking part of the reserve to a new stake account, the next
    /// program-derived address for the stake account associated with the given
    /// validator, does not match the provided stake account, or the stake account
    /// is not the right account to stake with at this time.
    InvalidStakeAccount,

    /// We expected an SPL token account that holds stSOL,
    /// but this was not an SPL token account,
    /// or its mint did not match.
    InvalidStSolAccount,

    /// The exchange rate has already been updated this epoch.
    ExchangeRateAlreadyUpToDate,

    /// The exchange rate has not yet been updated this epoch.
    ExchangeRateNotUpdatedInThisEpoch,

    /// We observed a decrease in the balance of the validator's stake accounts.
    ValidatorBalanceDecreased,

    /// The provided stake authority does not match the one derived from Lido's state.
    InvalidStakeAuthority,

    /// The provided rewards withdraw authority does not match the one derived from Lido's state.
    InvalidRewardsWithdrawAuthority,

    /// The provided Vote Account is invalid or corrupted.
    InvalidVoteAccount,

    /// The provided token owner is different from the given one.
    InvalidTokenOwner,

    /// There is a validator that has more stake than the selected one.
    ValidatorWithMoreStakeExists,

    /// The provided mint is invalid.
    InvalidMint,

    /// Tried to deposit stake to inactive validator.
    StakeToInactiveValidator,

    /// Tried to remove a validator when it when it was active or had stake accounts.
    ValidatorIsStillActive,

    /// Tried to remove a validator when it when it was active or had stake accounts.
    ValidatorShouldHaveNoStakeAccounts,

    /// There is a validator that has less stake than the selected one, stake to that one instead.
    ValidatorWithLessStakeExists,

    /// Tried to remove a validator when it when it was active or had stake accounts.
    ValidatorShouldHaveNoUnstakeAccounts,

    /// The validator already has the maximum number of unstake accounts.
    ///
    /// We can't unstake more in this epoch, wait for stake to deactivate, close
    /// the unstake accounts with `WithdrawInactiveStake`, and retry next epoch.
    MaxUnstakeAccountsReached,

    /// The validator's vote account is not owned by the vote program.
    ValidatorVoteAccountHasDifferentOwner,

    /// We expected the StSol account to be owned by the SPL token program.
    InvalidStSolAccountOwner,
}

impl From<ArithmeticError> for LidoError {
    fn from(_: ArithmeticError) -> Self {
        LidoError::CalculationFailure
    }
}

impl From<ArithmeticError> for anchor_lang::error::Error {
    fn from(_: ArithmeticError) -> Self {
        error!(LidoError::CalculationFailure)
    }
}