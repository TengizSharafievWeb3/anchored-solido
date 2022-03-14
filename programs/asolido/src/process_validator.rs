use crate::AddValidator;
use anchor_lang::prelude::*;
use crate::state::Validator;

impl<'info> AddValidator<'info> {
    pub fn process(&mut self) -> Result<()> {
        let lido = &mut self.lido;
        lido.validators.add(
            self.validator_vote.key(),
            Validator::new(self.validator_fee_st_sol.key()))
            .map_err(|err| error!(err))
    }
}
