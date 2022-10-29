import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Launchpad } from "../target/types/launchpad";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  AccountMeta,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import * as spl from "@solana/spl-token";
import { BN } from "bn.js";

const NUM_TOKENS = 2;

export class LaunchpadTester {
  provider: anchor.AnchorProvider;
  program: anchor.Program<Launchpad>;
  printErrors: boolean;

  admins: Keypair[];
  feesAccount: PublicKey;
  adminMetas: AccountMeta[];

  // pdas
  multisig: [PublicKey, number];
  authority: [PublicKey, number];
  launchpad: [PublicKey, number];
  auction: [PublicKey, number];

  pricingCustody: {
    mint: Keypair;
    tokenAccount: PublicKey;
    oracleAccount: PublicKey;
    custody: PublicKey;
    decimals: number;
  };
  paymentCustody: {
    mint: Keypair;
    tokenAccount: PublicKey;
    oracleAccount: PublicKey;
    custody: PublicKey;
    decimals: number;
  };
  dispensingCustodies: {
    mint: Keypair;
    tokenAccount: PublicKey;
    decimals: number;
  }[];
  dispensingMetas: AccountMeta[];

  users: {
    wallet: Keypair;
    paymentAccount: PublicKey;
    receivingAccounts: PublicKey[];
  }[];
  seller: {
    wallet: Keypair;
    paymentAccount: PublicKey;
    dispensingAccounts: PublicKey[];
  };

  constructor() {
    this.provider = anchor.AnchorProvider.env();
    anchor.setProvider(this.provider);
    this.program = anchor.workspace.Launchpad as Program<Launchpad>;
    this.printErrors = true;

    // fixed addresses
    this.admins = [];
    this.admins.push(Keypair.generate());
    this.admins.push(Keypair.generate());

    this.adminMetas = [];
    for (const admin of this.admins) {
      this.adminMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: admin.publicKey,
      });
    }

    anchor.BN.prototype.toJSON = function () {
      return this.toString(10);
    };
  }

  init_fixture = async () => {
    // pdas
    this.multisig = await this.findProgramAddress("multisig");
    this.authority = await this.findProgramAddress("transfer_authority");
    this.launchpad = await this.findProgramAddress("launchpad");
    this.auction = await this.findProgramAddress("auction", "test auction");

    // custodies
    this.pricingCustody = await this.generateCustody(9);
    this.paymentCustody = await this.generateCustody(6);

    this.dispensingCustodies = [];
    this.dispensingMetas = [];
    for (let i = 0; i < NUM_TOKENS; ++i) {
      let mint = Keypair.generate();
      let tokenAccount = (
        await this.findProgramAddress("dispense", [
          mint.publicKey,
          this.auction[0],
        ])
      )[0];
      this.dispensingCustodies.push({
        mint: mint,
        tokenAccount: tokenAccount,
        decimals: 8,
      });
      this.dispensingMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: tokenAccount,
      });
    }

    // airdrop funds
    await this.confirmTx(await this.requestAirdrop(this.admins[0].publicKey));

    // create mints
    await spl.createMint(
      this.provider.connection,
      this.admins[0],
      this.admins[0].publicKey,
      null,
      this.pricingCustody.decimals,
      this.pricingCustody.mint
    );

    await spl.createMint(
      this.provider.connection,
      this.admins[0],
      this.admins[0].publicKey,
      null,
      this.paymentCustody.decimals,
      this.paymentCustody.mint
    );

    for (const custody of this.dispensingCustodies) {
      await spl.createMint(
        this.provider.connection,
        this.admins[0],
        this.admins[0].publicKey,
        null,
        custody.decimals,
        custody.mint
      );
    }

    // fees receiving account
    this.feesAccount = await spl.createAssociatedTokenAccount(
      this.provider.connection,
      this.admins[0],
      this.paymentCustody.mint.publicKey,
      this.admins[0].publicKey
    );

    // users
    this.users = [];
    for (let i = 0; i < 2; ++i) {
      let wallet = Keypair.generate();
      await this.requestAirdrop(wallet.publicKey);

      let paymentAccount = await spl.createAssociatedTokenAccount(
        this.provider.connection,
        this.admins[0],
        this.paymentCustody.mint.publicKey,
        wallet.publicKey
      );

      let receivingAccounts = [];
      for (const custody of this.dispensingCustodies) {
        receivingAccounts.push(
          await spl.createAssociatedTokenAccount(
            this.provider.connection,
            this.admins[0],
            custody.mint.publicKey,
            wallet.publicKey
          )
        );
      }

      this.users.push({
        wallet: wallet,
        paymentAccount: paymentAccount,
        receivingAccounts: receivingAccounts,
      });
    }

    // seller
    let wallet = Keypair.generate();
    await this.requestAirdrop(wallet.publicKey);

    let paymentAccount = await spl.createAssociatedTokenAccount(
      this.provider.connection,
      this.admins[0],
      this.paymentCustody.mint.publicKey,
      wallet.publicKey
    );

    let dispensingAccounts = [];
    for (const custody of this.dispensingCustodies) {
      dispensingAccounts.push(
        await spl.createAssociatedTokenAccount(
          this.provider.connection,
          this.admins[0],
          custody.mint.publicKey,
          wallet.publicKey
        )
      );
    }

    this.seller = {
      wallet: wallet,
      paymentAccount: paymentAccount,
      dispensingAccounts: dispensingAccounts,
    };
  };

  requestAirdrop = async (pubkey) => {
    if ((await this.getBalance(pubkey)) < 1e9 / 2) {
      return this.provider.connection.requestAirdrop(pubkey, 1e9);
    }
  };

  generateCustody = async (decimals: number) => {
    let mint = Keypair.generate();
    let tokenAccount = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      this.authority[0],
      true
    );
    let oracleAccount = (
      await this.findProgramAddress("oracle_account", [
        mint.publicKey,
        this.auction[0],
      ])
    )[0];
    let custody = (
      await this.findProgramAddress("custody", [mint.publicKey])
    )[0];
    return {
      mint: mint,
      tokenAccount: tokenAccount,
      oracleAccount: oracleAccount,
      custody: custody,
      decimals: decimals,
    };
  };

  findProgramAddress = async (label, extra_seeds = null) => {
    let seeds = [Buffer.from(anchor.utils.bytes.utf8.encode(label))];
    if (extra_seeds) {
      for (let extra_seed of extra_seeds) {
        if (typeof extra_seed === "string") {
          seeds.push(Buffer.from(anchor.utils.bytes.utf8.encode(extra_seed)));
        } else {
          seeds.push(extra_seed.toBuffer());
        }
      }
    }
    return await PublicKey.findProgramAddress(seeds, this.program.programId);
  };

  confirmTx = async (txSignature: anchor.web3.TransactionSignature) => {
    const latestBlockHash = await this.provider.connection.getLatestBlockhash();

    await this.provider.connection.confirmTransaction(
      {
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: txSignature,
      },
      { commitment: "processed" }
    );
  };

  confirmAndLogTx = async (txSignature: anchor.web3.TransactionSignature) => {
    await this.confirmTx(txSignature);
    let tx = await this.provider.connection.getTransaction(txSignature, {
      commitment: "confirmed",
    });
    console.log(tx);
  };

  getBalance = async (pubkey: PublicKey) => {
    return spl
      .getAccount(this.provider.connection, pubkey)
      .then((account) => Number(account.amount))
      .catch(() => 0);
  };

  getTime() {
    const now = new Date();
    const utcMilllisecondsSinceEpoch =
      now.getTime() + now.getTimezoneOffset() * 60 * 1000;
    return utcMilllisecondsSinceEpoch / 1000;
  }

  ensureFails = async (promise) => {
    let printErrors = this.printErrors;
    this.printErrors = false;
    let res = null;
    try {
      await promise;
    } catch (err) {
      res = err;
    }
    this.printErrors = printErrors;
    if (!res) {
      throw new Error("Call should've failed");
    }
    return res;
  };

  ///////
  // instructions

  init = async () => {
    try {
      await this.program.methods
        .testInit({
          minSignatures: 2,
          allowNewAuctions: true,
          allowAuctionUpdates: true,
          allowNewBids: true,
          allowWithdrawals: true,
          newAuctionFee: { numerator: new BN(1), denominator: new BN(100) },
          auctionUpdateFee: { numerator: new BN(1), denominator: new BN(100) },
          invalidBidFee: { numerator: new BN(1), denominator: new BN(100) },
          tradeFee: { numerator: new BN(1), denominator: new BN(100) },
        })
        .accounts({
          upgradeAuthority: this.provider.wallet.publicKey,
          multisig: this.multisig[0],
          transferAuthority: this.authority[0],
          launchpad: this.launchpad[0],
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(this.adminMetas)
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  setAdminSigners = async (minSignatures: number) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setAdminSigners({
            minSignatures: minSignatures,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            systemProgram: SystemProgram.programId,
          })
          .remainingAccounts(this.adminMetas)
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setFees = async (fees) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setFees(fees)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            launchpad: this.launchpad[0],
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setPermissions = async (permissions) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setPermissions(permissions)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            launchpad: this.launchpad[0],
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setOracleConfig = async (config, custody) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setOracleConfig(config)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            custody: custody.custody,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  initCustody = async (config, custody) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .initCustody(config)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            transferAuthority: this.authority[0],
            custody: custody.custody,
            custodyTokenMint: custody.mint.publicKey,
            custodyTokenAccount: custody.tokenAccount,
            systemProgram: SystemProgram.programId,
            rent: SYSVAR_RENT_PUBKEY,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
            associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  withdrawFees = async (amount, custody) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .withdrawFees({ amount: amount })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            transferAuthority: this.authority[0],
            launchpad: this.launchpad[0],
            custody: custody.custody,
            custodyTokenAccount: custody.tokenAccount,
            receivingAccount: this.feesAccount,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  deleteAuction = async () => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .deleteAuction({})
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            auction: this.auction[0],
            tokenProgram: spl.TOKEN_PROGRAM_ID,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setTestOraclePrice = async (price: number, custody) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setTestOraclePrice({
            price: new BN(price * 1000),
            expo: -3,
            conf: new BN(0),
            publishTime: new BN(this.getTime()),
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            auction: this.auction[0],
            custody: custody.custody,
            oracleAccount: custody.oracleAccount,
            systemProgram: SystemProgram.programId,
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  setTestTime = async (time) => {
    let multisig = await this.program.account.multisig.fetch(this.multisig[0]);
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setTestTime({
            time: time,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig[0],
            auction: this.auction[0],
          })
          .signers([this.admins[i]])
          .rpc();
      } catch (err) {
        if (this.printErrors) {
          console.log(err);
        }
        throw err;
      }
    }
  };

  initAuction = async (params) => {
    try {
      await this.program.methods
        .initAuction(params)
        .accounts({
          owner: this.seller.wallet.publicKey,
          launchpad: this.launchpad[0],
          auction: this.auction[0],
          pricingCustody: this.pricingCustody.custody,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(this.dispensingMetas)
        .signers([this.seller.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  updateAuction = async (params) => {
    try {
      await this.program.methods
        .updateAuction(params)
        .accounts({
          owner: this.seller.wallet.publicKey,
          launchpad: this.launchpad[0],
          auction: this.auction[0],
        })
        .signers([this.seller.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  disableAuction = async () => {
    try {
      await this.program.methods
        .disableAuction({})
        .accounts({
          owner: this.seller.wallet.publicKey,
          auction: this.auction[0],
        })
        .signers([this.seller.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  enableAuction = async () => {
    try {
      await this.program.methods
        .enableAuction({})
        .accounts({
          owner: this.seller.wallet.publicKey,
          auction: this.auction[0],
        })
        .signers([this.seller.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };
}
