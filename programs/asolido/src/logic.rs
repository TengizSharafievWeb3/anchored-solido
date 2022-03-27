use anchor_lang::context::CpiContext;
use anchor_lang::Key;
use anchor_lang::prelude::{Account, Result};
use solana_program::account_info::AccountInfo;
use solana_program::pubkey::Pubkey;
use crate::{Lido, MINT_AUTHORITY, StLamports};

/// Mint the given amount of stSOL and put it in the recipient's account.
///
/// * The stSOL mint must be the one configured in the Solido instance.
/// * The recipient account must be an stSOL SPL token account.
pub fn mint_st_sol_to<'a>(
    solido: &Box<Account<Lido>>,
    spl_token_program: AccountInfo<'a>,
    st_sol_mint: AccountInfo<'a>,
    mint_authority: AccountInfo<'a>,
    recipient: AccountInfo<'a>,
    amount: StLamports,
) -> Result<()> {
    let pubkey = solido.key();

    let authority_signature_seeds = [
        pubkey.as_ref(),
        MINT_AUTHORITY.as_ref(),
        &[solido.mint_authority_bump_seed],
    ];
    let signers = [&authority_signature_seeds[..]];

    let cpi_accounts = anchor_spl::token::MintTo {
        mint: st_sol_mint,
        to: recipient,
        authority: mint_authority,
    };

    let cpi_context = CpiContext::new_with_signer(
        spl_token_program,
        cpi_accounts,
        &signers,
    );

    anchor_spl::token::mint_to(cpi_context, amount.amount)
}