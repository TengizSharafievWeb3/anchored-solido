import * as anchor from "@project-serum/anchor";
import {Program, web3} from "@project-serum/anchor";
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

  const lido = anchor.web3.Keypair.generate();
  const manager = anchor.web3.Keypair.generate();
  const st_sol_mint = anchor.web3.Keypair.generate();

  const node = anchor.web3.Keypair.generate();
  const fee = anchor.web3.Keypair.generate();
  const vote = anchor.web3.Keypair.generate();

  async function create_mint(mint: anchor.web3.Keypair, mint_authority: anchor.web3.PublicKey) {
    await spl_token.methods
      .initializeMint(9, mint_authority, null)
      .accounts({
        mint: st_sol_mint.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([st_sol_mint])
      .preInstructions([await spl_token.account.mint.createInstruction(st_sol_mint)])
      .rpc();
  }

  async function create_token(token: anchor.web3.Keypair, mint: anchor.web3.PublicKey, authority: anchor.web3.PublicKey) {
    await spl_token.methods.initializeAccount()
      .accounts({
        account: token.publicKey,
        mint: st_sol_mint.publicKey,
        authority: provider.wallet.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([token])
      .preInstructions([await spl_token.account.token.createInstruction(token)])
      .rpc();
  }

  async function create_vote(vote: anchor.web3.Keypair, node: anchor.web3.Keypair, authorizedWithdrawer: anchor.web3.PublicKey, commission: number) {
    const rent_voter = await provider.connection.getMinimumBalanceForRentExemption(anchor.web3.VoteProgram.space);
    const minimum = await provider.connection.getMinimumBalanceForRentExemption(0);
    await provider.send(
      new anchor.web3.Transaction()
        .add(anchor.web3.SystemProgram.createAccount({
          fromPubkey: provider.wallet.publicKey,
          newAccountPubkey: node.publicKey,
          programId: anchor.web3.SystemProgram.programId,
          lamports: minimum,
          space: 0
        }))
        .add(anchor.web3.VoteProgram.createAccount({
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
    const treasury = anchor.web3.Keypair.generate();
    const developer = anchor.web3.Keypair.generate();

    const [mint_authority, _nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    await create_mint(st_sol_mint, mint_authority);
    await create_token(treasury, st_sol_mint.publicKey, provider.wallet.publicKey);
    await create_token(developer, st_sol_mint.publicKey, provider.wallet.publicKey);
    await create_token(fee, st_sol_mint.publicKey, provider.wallet.publicKey);

    // derive & fund reserve
    const [reserve, _reserve_nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("reserve_account"))], program.programId);

    const min_balance = await provider.connection.getMinimumBalanceForRentExemption(0);
    await provider.send(
      new anchor.web3.Transaction()
        .add(anchor.web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: reserve,
          lamports: min_balance
        })));

    const [withrawer, _withrawer_nonce] = await anchor.web3.PublicKey.findProgramAddress(
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
    const owner = anchor.web3.Keypair.generate();
    const rent_voter = await provider.connection.getMinimumBalanceForRentExemption(anchor.web3.VoteProgram.space);
    const invalid_vote = anchor.web3.Keypair.generate();

    await expect(program.methods.addValidator()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        validator_vote: invalid_vote.publicKey,
      })
      .signers([manager, invalid_vote])
      .preInstructions([anchor.web3.SystemProgram.createAccount({
        fromPubkey: provider.wallet.publicKey,
        newAccountPubkey: invalid_vote.publicKey,
        programId: owner.publicKey,
        lamports: rent_voter,
        space: anchor.web3.VoteProgram.space,
      })])
      .rpc()).to.be.rejected;
  });
});