import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { AnchoredSolido } from "../target/types/anchored_solido";

describe("anchored-solido", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.AnchoredSolido as Program<AnchoredSolido>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
