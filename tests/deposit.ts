import * as anchor from "@project-serum/anchor";
import {Program, web3, BN} from "@project-serum/anchor";
import {PublicKey, Keypair} from '@solana/web3.js';
import {Asolido} from "../target/types/asolido";

import {expect} from 'chai';
import * as chai from 'chai';
import chaiAsPromised from 'chai-as-promised';

chai.use(chaiAsPromised);

describe("Deposit", () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.Asolido as Program<Asolido>;
  const spl_token = anchor.Spl.token();

  const lido = Keypair.generate();
  const manager = Keypair.generate();
  const st_sol_mint = Keypair.generate();

  const maintainer = Keypair.generate();

  const node = Keypair.generate();
  const fee = Keypair.generate();
  const vote = Keypair.generate();

  const TEST_DEPOSIT_AMOUNT = 100000000;

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

  async function fund(to: PublicKey, amount: number) {
    await provider.send(
      new web3.Transaction()
        .add(web3.SystemProgram.transfer(
      {
        fromPubkey: provider.wallet.publicKey,
        toPubkey: to,
        lamports: amount + await provider.connection.getMinimumBalanceForRentExemption(0),
      })
    ));
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

    await program.methods.addValidator()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        validatorVote: vote.publicKey,
        validatorFeeStSol: fee.publicKey,
      })
      .signers([manager])
      .rpc();

    await program.methods.addMaintainer()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        maintainer: maintainer.publicKey,
      })
      .signers([manager])
      .rpc();
  });

  it("Should deposit", async () => {
    // Create a new user who is going to do the deposit. The user's account
    // will hold the SOL to deposit, and it will also be the owner of the
    // stSOL account that holds the proceeds.
    const user = Keypair.generate();
    const recipient = Keypair.generate();
    await create_token(recipient, st_sol_mint.publicKey, user.publicKey);
    await fund(user.publicKey, TEST_DEPOSIT_AMOUNT);

    await program.methods
      .deposit(new BN(TEST_DEPOSIT_AMOUNT))
      .accounts({
        lido: lido.publicKey,
        user: user.publicKey,
        recipient: recipient.publicKey,
        stSolMint: st_sol_mint.publicKey,
      })
      .signers([user])
      .rpc();

    const [reserve, _reserve_nonce] = await PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("reserve_account"))], program.programId);

    const reserveBalance = await provider.connection.getBalance(reserve);
    const rentExempt = await provider.connection.getMinimumBalanceForRentExemption(0);

    // In general, the received stSOL need not be equal to the deposited SOL,
    // but initially, the exchange rate is 1, so this holds.
    expect(reserveBalance).to.be.equal(rentExempt + TEST_DEPOSIT_AMOUNT);
    const recipientAccount = program.account.recipient.fetch(recipient.publicKey);
    expect(recipientAccount.amount).to.be.deep.equal(new BN(TEST_DEPOSIT_AMOUNT));

    const lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.metrics.depositAmount.total).to.be.deep.equal(new BN(TEST_DEPOSIT_AMOUNT));
  });
});