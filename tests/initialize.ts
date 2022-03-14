import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { MintLayout, Token, } from '@solana/spl-token';
import { Asolido } from "../target/types/asolido";

import { expect } from 'chai';
import * as chai from 'chai';
import chaiAsPromised from 'chai-as-promised';
chai.use(chaiAsPromised);

describe("Initialize anchored-solido", () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const provider = anchor.getProvider();
  const program = anchor.workspace.Asolido as Program<Asolido>;
  const spl_token = anchor.Spl.token();
  const rent = anchor.web3.SYSVAR_RENT_PUBKEY;

  const lido = anchor.web3.Keypair.generate();
  const manager = anchor.web3.Keypair.generate();
  const st_sol_mint = anchor.web3.Keypair.generate();
  const treasury = anchor.web3.Keypair.generate();
  const developer = anchor.web3.Keypair.generate();

  before(async () => {
    const [mint_authority, _nonce] = await anchor.web3.PublicKey.findProgramAddress(
        [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

    // Create mint
    await spl_token.methods
        .initializeMint(0, mint_authority, null)
        .accounts({
          mint: st_sol_mint.publicKey,
          rent,
        })
        .signers([st_sol_mint])
        .preInstructions([await spl_token.account.mint.createInstruction(st_sol_mint)])
        .rpc();

    // Create treasury
    await spl_token.methods.initializeAccount()
        .accounts({
          account: treasury.publicKey,
          mint: st_sol_mint.publicKey,
          authority: provider.wallet.publicKey,
          rent,
        })
        .signers([treasury])
        .preInstructions([await spl_token.account.token.createInstruction(treasury)])
        .rpc();

    // Create developer
    await spl_token.methods.initializeAccount()
        .accounts({
          account: developer.publicKey,
          mint: st_sol_mint.publicKey,
          authority: provider.wallet.publicKey,
          rent,
        })
        .signers([developer])
        .preInstructions([await spl_token.account.token.createInstruction(developer)])
        .rpc();

    // derive & fund reserve
    const [reserve, _reserve_nonce] = await anchor.web3.PublicKey.findProgramAddress(
          [lido.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("reserve_account"))], program.programId);
    const min_balance = await provider.connection.getMinimumBalanceForRentExemption(0);
    await provider.send(
          new anchor.web3.Transaction()
              .add(anchor.web3.SystemProgram.transfer({fromPubkey: provider.wallet.publicKey, toPubkey: reserve, lamports: min_balance})));
  });

  it("Should initialize", async () => {
    const max_validators = 10000;
    const max_maintainers = 1000;

    await program.methods
        .initialize({treasuryFee: 5, validationFee: 3, developerFee: 2, stSolAppreciation: 90}, max_validators, max_maintainers)
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

  it("Should NOT initialize with incorrect mint", async () => {
      const lido1 = anchor.web3.Keypair.generate();
      const st_sol_mint1 = anchor.web3.Keypair.generate();
      const treasury1 = anchor.web3.Keypair.generate();
      const developer1 = anchor.web3.Keypair.generate();

      // Create mint with incorrect mint authority
      await spl_token.methods
          .initializeMint(0, provider.wallet.publicKey, null)
          .accounts({
              mint: st_sol_mint1.publicKey,
              rent,
          })
          .signers([st_sol_mint1])
          .preInstructions([await spl_token.account.mint.createInstruction(st_sol_mint1)])
          .rpc();

      // Create treasury
      await spl_token.methods.initializeAccount()
          .accounts({
              account: treasury1.publicKey,
              mint: st_sol_mint1.publicKey,
              authority: provider.wallet.publicKey,
              rent,
          })
          .signers([treasury1])
          .preInstructions([await spl_token.account.token.createInstruction(treasury1)])
          .rpc();

      // Create developer
      await spl_token.methods.initializeAccount()
          .accounts({
              account: developer1.publicKey,
              mint: st_sol_mint1.publicKey,
              authority: provider.wallet.publicKey,
              rent,
          })
          .signers([developer1])
          .preInstructions([await spl_token.account.token.createInstruction(developer1)])
          .rpc();

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
        const lido1 = anchor.web3.Keypair.generate();
        const st_sol_mint1 = anchor.web3.Keypair.generate();
        const treasury1 = anchor.web3.Keypair.generate();
        const developer1 = anchor.web3.Keypair.generate();

        const [mint_authority, _nonce] = await anchor.web3.PublicKey.findProgramAddress(
            [lido1.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

        // Create mint
        await spl_token.methods
            .initializeMint(0, mint_authority, null)
            .accounts({
                mint: st_sol_mint1.publicKey,
                rent,
            })
            .signers([st_sol_mint1])
            .preInstructions([await spl_token.account.mint.createInstruction(st_sol_mint1)])
            .rpc();

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

    it("Should NOT initialize with not funded reserve account", async() => {
        const lido1 = anchor.web3.Keypair.generate();
        const st_sol_mint1 = anchor.web3.Keypair.generate();
        const treasury1 = anchor.web3.Keypair.generate();
        const developer1 = anchor.web3.Keypair.generate();

        const [mint_authority, _nonce] = await anchor.web3.PublicKey.findProgramAddress(
            [lido1.publicKey.toBuffer(), Buffer.from(anchor.utils.bytes.utf8.encode("mint_authority"))], program.programId);

        // Create mint
        await spl_token.methods
            .initializeMint(0, mint_authority, null)
            .accounts({
                mint: st_sol_mint1.publicKey,
                rent,
            })
            .signers([st_sol_mint1])
            .preInstructions([await spl_token.account.mint.createInstruction(st_sol_mint1)])
            .rpc();

        // Create treasury
        await spl_token.methods.initializeAccount()
            .accounts({
                account: treasury1.publicKey,
                mint: st_sol_mint1.publicKey,
                authority: provider.wallet.publicKey,
                rent,
            })
            .signers([treasury1])
            .preInstructions([await spl_token.account.token.createInstruction(treasury1)])
            .rpc();

        // Create developer
        await spl_token.methods.initializeAccount()
            .accounts({
                account: developer1.publicKey,
                mint: st_sol_mint1.publicKey,
                authority: provider.wallet.publicKey,
                rent,
            })
            .signers([developer1])
            .preInstructions([await spl_token.account.token.createInstruction(developer1)])
            .rpc();

        const max_validators = 10000;
        const max_maintainers = 1000;

        await expect(program.methods
            .initialize({treasuryFee: 5, validationFee: 3, developerFee: 2, stSolAppreciation: 90}, max_validators, max_maintainers)
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
