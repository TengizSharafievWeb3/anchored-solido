use crate::state::Validator;
use crate::{AddValidator, RemoveValidator};
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
