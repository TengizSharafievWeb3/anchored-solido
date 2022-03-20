import * as anchor from "@project-serum/anchor";
import {Program, web3, BN} from "@project-serum/anchor";
import {PublicKey, Keypair} from '@solana/web3.js';
import {Asolido} from "../target/types/asolido";

import {expect} from 'chai';
import * as chai from 'chai';
import chaiAsPromised from 'chai-as-promised';

chai.use(chaiAsPromised);

describe("Initialize anchored-solido", () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.Asolido as Program<Asolido>;
  const spl_token = anchor.Spl.token();
  const rent = web3.SYSVAR_RENT_PUBKEY;

  const lido = Keypair.generate();
  const manager = Keypair.generate();
  const st_sol_mint = Keypair.generate();
  const treasury = Keypair.generate();
  const developer = Keypair.generate();

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

  async function fund_reserve(lido: PublicKey) {
    const [reserve, _reserve_nonce] = await PublicKey.findProgramAddress(
      [lido.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("reserve_account"))], program.programId);
    await provider.send(
      new web3.Transaction()
        .add(web3.SystemProgram.transfer({
          fromPubkey: provider.wallet.publicKey,
          toPubkey: reserve,
          lamports: await provider.connection.getMinimumBalanceForRentExemption(0)
        })));
  }

  before(async () => {
    const [mint_authority, _nonce] = await PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    // Create mint
    await create_mint(st_sol_mint, mint_authority);
    // Create treasury
    await create_token(treasury, st_sol_mint.publicKey, provider.wallet.publicKey);
    // Create developer
    await create_token(developer, st_sol_mint.publicKey, provider.wallet.publicKey);
    // fund reserve
    await fund_reserve(lido.publicKey);
  });

  it("Should initialize", async () => {
    const max_validators = 10000;
    const max_maintainers = 1000;

    await program.methods
      .initialize({
        treasuryFee: 5,
        validationFee: 3,
        developerFee: 2,
        stSolAppreciation: 90
      }, max_validators, max_maintainers)
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        stSolMint: st_sol_mint.publicKey,
        treasury: treasury.publicKey,
        developer: developer.publicKey,
      })
      .signers([lido])
      .rpc();

    const lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.manager).to.be.deep.equal(manager.publicKey);
    expect(lidoAccount.stSolMint).to.be.deep.equal(st_sol_mint.publicKey);
    expect(lidoAccount.feeRecipients.treasuryAccount).to.be.deep.equal(treasury.publicKey);
    expect(lidoAccount.feeRecipients.developerAccount).to.be.deep.equal(developer.publicKey);

  });

  it("Should NOT initialize with incorrect mint", async () => {
    const lido1 = Keypair.generate();
    const st_sol_mint1 = Keypair.generate();
    const treasury1 = Keypair.generate();
    const developer1 = Keypair.generate();

    // Create mint with incorrect mint authority
    await create_mint(st_sol_mint1, provider.wallet.publicKey);
    // Create treasury
    await create_token(treasury1, st_sol_mint1.publicKey, provider.wallet.publicKey);
    // Create developer
    await create_token(developer1, st_sol_mint1.publicKey, provider.wallet.publicKey);

    await expect(program.methods
      .initialize({treasuryFee: 5, validationFee: 3, developerFee: 2, stSolAppreciation: 90}, 10000, 1000)
      .accounts({
        lido: lido1.publicKey,
        manager: manager.publicKey,
        stSolMint: st_sol_mint1.publicKey,
        treasury: treasury1.publicKey,
        developer: developer1.publicKey,
      })
      .signers([lido1])
      .rpc()).to.be.rejectedWith(/InvalidMint/);
  });

  it("Should NOT initialize with incorrect treasury", async () => {
    const lido1 = Keypair.generate();
    const st_sol_mint1 = Keypair.generate();
    const treasury1 = Keypair.generate();
    const developer1 = Keypair.generate();

    const [mint_authority, _nonce] = await PublicKey.findProgramAddress(
      [lido1.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    // Create mint
    await create_mint(st_sol_mint1, mint_authority);

    await expect(program.methods
      .initialize({treasuryFee: 5, validationFee: 3, developerFee: 2, stSolAppreciation: 90}, 10000, 1000)
      .accounts({
        lido: lido1.publicKey,
        manager: manager.publicKey,
        stSolMint: st_sol_mint1.publicKey,
        treasury: treasury.publicKey,
        developer: developer.publicKey,
      })
      .signers([lido1])
      .rpc()).to.be.rejectedWith(/InvalidFeeRecipient/);
  });

  it("Should NOT initialize with not funded reserve account", async () => {
    const lido1 = Keypair.generate();
    const st_sol_mint1 = Keypair.generate();
    const treasury1 = Keypair.generate();
    const developer1 = Keypair.generate();

    const [mint_authority, _nonce] = await PublicKey.findProgramAddress(
      [lido1.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    // Create mint
    await create_mint(st_sol_mint1, mint_authority);
    // Create treasury
    await create_token(treasury1, st_sol_mint1.publicKey, provider.wallet.publicKey);
    // Create developer
    await create_token(developer1, st_sol_mint1.publicKey, provider.wallet.publicKey);
    
    const max_validators = 10000;
    const max_maintainers = 1000;

    await expect(program.methods
      .initialize({
        treasuryFee: 5,
        validationFee: 3,
        developerFee: 2,
        stSolAppreciation: 90
      }, max_validators, max_maintainers)
      .accounts({
        lido: lido1.publicKey,
        manager: manager.publicKey,
        stSolMint: st_sol_mint1.publicKey,
        treasury: treasury1.publicKey,
        developer: developer1.publicKey,
      })
      .signers([lido1])
      .rpc()).to.be.rejected;
  });
});
