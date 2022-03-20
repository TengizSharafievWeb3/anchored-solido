import * as anchor from "@project-serum/anchor";
import {Program, web3, BN} from "@project-serum/anchor";
import {PublicKey, Keypair} from '@solana/web3.js';
import {Asolido} from "../target/types/asolido";

import {expect} from 'chai';
import * as chai from 'chai';
import chaiAsPromised from 'chai-as-promised';

chai.use(chaiAsPromised);

describe("Add Remove Validator", () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.Asolido as Program<Asolido>;
  const spl_token = anchor.Spl.token();

  const lido = Keypair.generate();
  const manager = Keypair.generate();
  const st_sol_mint = Keypair.generate();

  const node = Keypair.generate();
  const fee = Keypair.generate();
  const vote = Keypair.generate();

  async function create_mint(mint: Keypair, mint_authority: PublicKey) {
    await spl_token.methods
      .initializeMint(9, mint_authority, null)
      .accounts({
        mint: mint.publicKey,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([mint])
      .preInstructions([await spl_token.account.mint.createInstruction(mint)])
      .rpc();
  }

  async function create_token(token: Keypair, mint: PublicKey, authority: PublicKey) {
    await spl_token.methods.initializeAccount()
      .accounts({
        account: token.publicKey,
        mint: mint,
        authority: authority,
        rent: web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([token])
      .preInstructions([await spl_token.account.token.createInstruction(token)])
      .rpc();
  }

  async function create_vote(vote: Keypair, node: Keypair, authorizedWithdrawer: PublicKey, commission: number) {
    const rent_voter = await provider.connection.getMinimumBalanceForRentExemption(web3.VoteProgram.space);
    const minimum = await provider.connection.getMinimumBalanceForRentExemption(0);
    await provider.send(
      new web3.Transaction()
        .add(web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: node.publicKey,
          programId: web3.SystemProgram.programId,
          lamports: minimum,
          space: 0
        }))
        .add(web3.VoteProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          votePubkey: vote.publicKey,
          voteInit: {
            commission: commission,
            nodePubkey: node.publicKey,
            authorizedWithdrawer: authorizedWithdrawer,
            authorizedVoter: node.publicKey,
          },
          lamports: rent_voter,
        })),
      [node, vote]
    )
  }

  before(async () => {
    const treasury = Keypair.generate();
    const developer = Keypair.generate();

    const [mint_authority, _nonce] = await PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    await create_mint(st_sol_mint, mint_authority);
    await create_token(treasury, st_sol_mint.publicKey, provider.wallet.publicKey);
    await create_token(developer, st_sol_mint.publicKey, provider.wallet.publicKey);
    await create_token(fee, st_sol_mint.publicKey, provider.wallet.publicKey);
    
    const [withrawer, _withrawer_nonce] = await PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("rewards_withdraw_authority"))], program.programId);
    await create_vote(vote, node, withrawer, 100);

    // Initialize Lido
    await program.methods
      .initialize({treasuryFee: 5, validationFee: 3, developerFee: 2, stSolAppreciation: 90}, 10000, 1000)
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        stSolMint: st_sol_mint.publicKey,
        treasury: treasury.publicKey,
        developer: developer.publicKey,
      })
      .signers([lido])
      .rpc();
  });

  it("Should add validator", async () => {
    program.methods.addValidator()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        validator_vote: vote.publicKey,
      })
      .signers([manager])
      .rpc();
  });

  // Adding the validator a second time should fail.
  it("Should NOT add the same validator a second time", async () => {
    await expect(program.methods.addValidator()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        validator_vote: vote.publicKey,
      })
      .signers([manager])
      .rpc()).to.be.rejected;
  });

  // test_add_validator_with_invalid_owner
  it("Should NOT add validator with invalid owner", async () => {
    const owner = Keypair.generate();
    const rent_voter = await provider.connection.getMinimumBalanceForRentExemption(web3.VoteProgram.space);
    const invalid_vote = Keypair.generate();

    await expect(program.methods.addValidator()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        validator_vote: invalid_vote.publicKey,
      })
      .signers([manager, invalid_vote])
      .preInstructions([web3.SystemProgram.createAccount({
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: invalid_vote.publicKey,
        programId: owner.publicKey,
        lamports: rent_voter,
        space: web3.VoteProgram.space,
      })])
      .rpc()).to.be.rejected;
  });
});