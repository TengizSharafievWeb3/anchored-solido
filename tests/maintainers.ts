import * as anchor from "@project-serum/anchor";
import {Program, web3, BN} from "@project-serum/anchor";
import {PublicKey, Keypair} from '@solana/web3.js';
import {Asolido} from "../target/types/asolido";

import {expect} from 'chai';
import * as chai from 'chai';
import chaiAsPromised from 'chai-as-promised';

chai.use(chaiAsPromised);

describe("Maintainers", () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.Asolido as Program<Asolido>;
  const spl_token = anchor.Spl.token();

  const lido = Keypair.generate();
  const manager = Keypair.generate();
  const st_sol_mint = Keypair.generate();

  const maintainer1 = Keypair.generate();
  const maintainer2 = Keypair.generate();

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

  before(async () => {
    const treasury = Keypair.generate();
    const developer = Keypair.generate();

    const [mint_authority, _nonce] = await PublicKey.findProgramAddress(
      [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    await create_mint(st_sol_mint, mint_authority);
    await create_token(treasury, st_sol_mint.publicKey, provider.wallet.publicKey);
    await create_token(developer, st_sol_mint.publicKey, provider.wallet.publicKey);

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

  it("Should add maintainer", async () => {
    let lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.maintainers.entries.length).to.be.equal(0);

    await program.methods
      .addMaintainer()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        maintainer: maintainer1.publicKey,
      })
      .signers([manager])
      .rpc();

    lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.maintainers.entries.length).to.be.equal(1);
    expect(lidoAccount.maintainers.entries[0]).to.be.deep.equal(maintainer1.publicKey);
  });

  it("Should NOT add the same maintainer second time", async () => {
    const lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.maintainers.entries.length).to.be.equal(1);
    expect(lidoAccount.maintainers.entries[0]).to.be.deep.equal(maintainer1.publicKey);

    await expect(program.methods
      .addMaintainer()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        maintainer: maintainer1.publicKey,
      })
      .signers([manager])
      .rpc()).to.be.rejectedWith(/DuplicatedEntry/);
  });

  it("Should remove maintainer", async () => {
    let lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.maintainers.entries.length).to.be.equal(1);
    expect(lidoAccount.maintainers.entries[0]).to.be.deep.equal(maintainer1.publicKey);

    await program.methods.removeMaintainer()
      .accounts({
        lido: lido.publicKey,
        manager: manager.publicKey,
        maintainer: maintainer1.publicKey,
      })
      .signers([manager])
      .rpc();

    lidoAccount = await program.account.lido.fetch(lido.publicKey);
    expect(lidoAccount.maintainers.entries.length).to.be.equal(0);

  })

});