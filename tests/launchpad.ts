import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Launchpad } from "../target/types/launchpad";

describe("launchpad", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Launchpad as Program<Launchpad>;

  it("Is initialized!", async () => {
    // Add your test here.
    try {
      const tx = await program.methods
        .initialize()
        .accounts({
          recentSlothashes: new anchor.web3.PublicKey(
            "SysvarS1otHashes111111111111111111111111111"
          ),
        })
        .rpc();
      console.log("Your transaction signature", tx);
    } catch (err) {
      console.log(err);
    }
  });
});
