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

    // Create treasury
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

  // Positive
  // Negative
  // - Неправильный mint
  //   - Неправильный supply
  //   - Неправильный mint auth
  // - Неправильный treasury
  // - Неправильный develop
});
