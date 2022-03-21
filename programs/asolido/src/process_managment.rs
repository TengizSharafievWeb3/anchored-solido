use crate::state::Validator;
use crate::{AddMaintainer, AddValidator, DeactivateValidator, RemoveMaintainer, RemoveValidator};
use anchor_lang::prelude::*;

impl<'info> AddValidator<'info> {
    pub fn process(&mut self) -> Result<()> {
        let lido = &mut self.lido;
        lido.validators
            .add(
                self.validator_vote.key(),
                Validator::new(self.validator_fee_st_sol.key()),
            )
            .map_err(|err| error!(err))
    }
}

impl<'info> RemoveValidator<'info> {
    /// Remove a validator.
    ///
    /// This instruction is the final cleanup step in the validator removal process,
    /// and it is callable by anybody. Initiation of the removal (`DeactivateValidator`)
    /// is restricted to the manager, but once a validator is inactive, and there is
    /// no more stake delegated to it, removing it from the list can be done by anybody.
    pub fn process(&mut self) -> Result<()> {
        let removed_validator = self.lido.validators.remove(&self.validator_vote.key())?;
        removed_validator.check_can_be_removed()?;
        Ok(())
    }
}

/// Set the `active` flag to false for a given validator.
///
/// This prevents new funds from being staked with this validator, and enables
/// removing the validator once no stake is delegated to it any more, and once
/// it has no unclaimed fee credit.
impl<'info> DeactivateValidator<'info> {
    pub fn process(&mut self) -> Result<()> {
        let validator = self.lido.validators.get_mut(&self.validator_vote.key())?;
        validator.entry.active = false;
        // TODO Emit validator deactivated
        Ok(())
    }
}

impl<'info> AddMaintainer<'info> {
    pub fn process(&mut self) -> Result<()> {
        self.lido.maintainers.add(self.maintainer.key())
    }
}

impl<'info> RemoveMaintainer<'info> {
    pub fn process(&mut self) -> Result<()> {
        self.lido.maintainers.remove(&self.maintainer.key())
    }
}
