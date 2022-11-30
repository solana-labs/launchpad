import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { Launchpad } from "../target/types/launchpad";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  AccountMeta,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_SLOT_HASHES_PUBKEY,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  AddressLookupTableProgram,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import * as spl from "@solana/spl-token";
import { BN } from "bn.js";
import { token } from "@project-serum/anchor/dist/cjs/utils";

const NUM_TOKENS = 2;

export class LaunchpadTester {
  provider: anchor.AnchorProvider;
  program: anchor.Program<Launchpad>;
  printErrors: boolean;

  admins: Keypair[];
  feesAccount: PublicKey;
  adminMetas: AccountMeta[];

  // pdas
  multisig: { publicKey: PublicKey; bump: number };
  authority: { publicKey: PublicKey; bump: number };
  launchpad: { publicKey: PublicKey; bump: number };
  auction: { publicKey: PublicKey; bump: number };
  lookupTable: { publicKey: PublicKey; bump: number };

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
    bump: number;
    decimals: number;
  }[];
  dispensingAccountMetas: AccountMeta[];
  dispensingMintMetas: AccountMeta[];

  users: {
    wallet: Keypair;
    paymentAccount: PublicKey;
    receivingAccountMetas: AccountMeta[];
  }[];
  seller: {
    wallet: Keypair;
    paymentAccount: PublicKey;
    balanceAccount: PublicKey;
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

  initFixture = async () => {
    // pdas
    this.multisig = await this.findProgramAddress("multisig");
    this.authority = await this.findProgramAddress("transfer_authority");
    this.launchpad = await this.findProgramAddress("launchpad");
    this.auction = await this.findProgramAddress("auction", "test auction");
    let slot = await this.provider.connection.getSlot();
    this.lookupTable = {
      publicKey: AddressLookupTableProgram.createLookupTable({
        authority: this.authority.publicKey,
        payer: this.provider.wallet.publicKey,
        recentSlot: slot,
      })[1],
      bump: slot,
    };

    // custodies
    this.pricingCustody = await this.generateCustody(9);
    this.paymentCustody = await this.generateCustody(6);

    this.dispensingCustodies = [];
    this.dispensingAccountMetas = [];
    this.dispensingMintMetas = [];
    for (let i = 0; i < NUM_TOKENS; ++i) {
      let mint = Keypair.generate();
      let tokenAccount = await this.findProgramAddress("dispense", [
        mint.publicKey,
        this.auction.publicKey,
      ]);
      this.dispensingCustodies.push({
        mint: mint,
        tokenAccount: tokenAccount.publicKey,
        bump: tokenAccount.bump,
        decimals: 8,
      });
      this.dispensingAccountMetas.push({
        isSigner: false,
        isWritable: true,
        pubkey: tokenAccount.publicKey,
      });
      this.dispensingMintMetas.push({
        isSigner: false,
        isWritable: false,
        pubkey: mint.publicKey,
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
      await this.mintTokens(
        1000,
        this.paymentCustody.decimals,
        this.paymentCustody.mint.publicKey,
        paymentAccount
      );

      let receivingAccountMetas = [];
      for (const custody of this.dispensingCustodies) {
        receivingAccountMetas.push({
          isSigner: false,
          isWritable: true,
          pubkey: await spl.createAssociatedTokenAccount(
            this.provider.connection,
            this.admins[0],
            custody.mint.publicKey,
            wallet.publicKey
          ),
        });
      }

      this.users.push({
        wallet: wallet,
        paymentAccount: paymentAccount,
        receivingAccountMetas: receivingAccountMetas,
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
      await this.mintTokens(
        1000,
        custody.decimals,
        custody.mint.publicKey,
        dispensingAccounts[dispensingAccounts.length - 1]
      );
    }

    let balanceAccount = (
      await this.findProgramAddress("seller_balance", [
        wallet.publicKey,
        this.paymentCustody.custody,
      ])
    ).publicKey;

    this.seller = {
      wallet: wallet,
      paymentAccount: paymentAccount,
      balanceAccount: balanceAccount,
      dispensingAccounts: dispensingAccounts,
    };
  };

  requestAirdrop = async (pubkey: PublicKey) => {
    if ((await this.getSolBalance(pubkey)) < 1e9 / 2) {
      return this.provider.connection.requestAirdrop(pubkey, 1e9);
    }
  };

  mintTokens = async (
    uiAmount: number,
    decimals: number,
    mint: PublicKey,
    destiantionWallet: PublicKey
  ) => {
    await spl.mintToChecked(
      this.provider.connection,
      this.admins[0],
      mint,
      destiantionWallet,
      this.admins[0],
      this.toTokenAmount(uiAmount, decimals).toNumber(),
      decimals
    );
  };

  generateCustody = async (decimals: number) => {
    let mint = Keypair.generate();
    let tokenAccount = await spl.getAssociatedTokenAddress(
      mint.publicKey,
      this.authority.publicKey,
      true
    );
    let oracleAccount = (
      await this.findProgramAddress("oracle_account", [
        mint.publicKey,
        this.auction.publicKey,
      ])
    ).publicKey;
    let custody = (await this.findProgramAddress("custody", [mint.publicKey]))
      .publicKey;
    return {
      mint: mint,
      tokenAccount: tokenAccount,
      oracleAccount: oracleAccount,
      custody: custody,
      decimals: decimals,
    };
  };

  findProgramAddress = async (label: string, extra_seeds = null) => {
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
    let res = await PublicKey.findProgramAddress(seeds, this.program.programId);
    return { publicKey: res[0], bump: res[1] };
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

  getSolBalance = async (pubkey: PublicKey) => {
    return this.provider.connection
      .getBalance(pubkey)
      .then((balance) => balance)
      .catch(() => 0);
  };

  getExtraSolBalance = async (pubkey: PublicKey) => {
    let balance = await this.provider.connection
      .getBalance(pubkey)
      .then((balance) => balance)
      .catch(() => 0);
    let accountInfo = await this.provider.connection.getAccountInfo(pubkey);
    let dataSize = accountInfo ? accountInfo.data.length : 0;
    let minBalance =
      await this.provider.connection.getMinimumBalanceForRentExemption(
        dataSize
      );
    return balance > minBalance ? balance - minBalance : 0;
  };

  getTokenAccount = async (pubkey: PublicKey) => {
    return spl.getAccount(this.provider.connection, pubkey);
  };

  getTime() {
    const now = new Date();
    const utcMilllisecondsSinceEpoch =
      now.getTime() + now.getTimezoneOffset() * 60 * 1000;
    return utcMilllisecondsSinceEpoch / 1000;
  }

  toTokenAmount(uiAmount: number, decimals: number) {
    return new BN(uiAmount * 10 ** decimals);
  }

  toUiAmount(token_amount: number, decimals: number) {
    return token_amount / 10 ** decimals;
  }

  getBidAddress = async (pubkey: PublicKey) => {
    return (
      await this.findProgramAddress("bid", [pubkey, this.auction.publicKey])
    ).publicKey;
  };

  ensureFails = async (promise, message = null) => {
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
      throw new Error(message ? message : "Call should've failed");
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
          allowAuctionRefills: true,
          allowAuctionPullouts: true,
          allowNewBids: true,
          allowWithdrawals: true,
          newAuctionFee: new BN(100),
          auctionUpdateFee: new BN(100),
          invalidBidFee: { numerator: new BN(1), denominator: new BN(100) },
          tradeFee: { numerator: new BN(1), denominator: new BN(100) },
          recentSlot: new BN(this.lookupTable.bump),
        })
        .accounts({
          upgradeAuthority: this.provider.wallet.publicKey,
          multisig: this.multisig.publicKey,
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          lookupTable: this.lookupTable.publicKey,
          recentSlothashes: SYSVAR_SLOT_HASHES_PUBKEY,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          lookupTableProgram: AddressLookupTableProgram.programId,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setAdminSigners({
            minSignatures: minSignatures,
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setFees(fees)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            launchpad: this.launchpad.publicKey,
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setPermissions(permissions)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            launchpad: this.launchpad.publicKey,
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setOracleConfig(config)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .initCustody(config)
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            custody: custody.custody,
            custodyTokenMint: custody.mint.publicKey,
            custodyTokenAccount: custody.tokenAccount,
            systemProgram: SystemProgram.programId,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
            associatedTokenProgram: spl.ASSOCIATED_TOKEN_PROGRAM_ID,
            rent: SYSVAR_RENT_PUBKEY,
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

  withdrawFees = async (
    tokenAmount: number,
    solAmount: number,
    custody,
    receivingTokenAccount,
    receivingSolAccount
  ) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .withdrawFees({
            tokenAmount: new BN(tokenAmount),
            solAmount: new BN(solAmount),
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            launchpad: this.launchpad.publicKey,
            custody: custody.custody,
            custodyTokenAccount: custody.tokenAccount,
            receivingTokenAccount: receivingTokenAccount,
            receivingSolAccount: receivingSolAccount,
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .deleteAuction({})
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            transferAuthority: this.authority.publicKey,
            launchpad: this.launchpad.publicKey,
            auction: this.auction.publicKey,
            tokenProgram: spl.TOKEN_PROGRAM_ID,
          })
          .remainingAccounts(this.dispensingAccountMetas)
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
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
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
            multisig: this.multisig.publicKey,
            auction: this.auction.publicKey,
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

  setTestTime = async (time: number) => {
    let multisig = await this.program.account.multisig.fetch(
      this.multisig.publicKey
    );
    for (let i = 0; i < multisig.minSignatures; ++i) {
      try {
        await this.program.methods
          .setTestTime({
            time: new BN(time),
          })
          .accounts({
            admin: this.admins[i].publicKey,
            multisig: this.multisig.publicKey,
            auction: this.auction.publicKey,
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
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          pricingCustody: this.pricingCustody.custody,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .remainingAccounts([
          ...this.dispensingAccountMetas,
          ...this.dispensingMintMetas,
        ])
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
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          systemProgram: SystemProgram.programId,
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
          auction: this.auction.publicKey,
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
          auction: this.auction.publicKey,
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

  addTokens = async (uiAmount: number, custodyId: number) => {
    try {
      await this.program.methods
        .addTokens({
          amount: this.toTokenAmount(
            uiAmount,
            this.dispensingCustodies[custodyId].decimals
          ),
        })
        .accounts({
          owner: this.seller.wallet.publicKey,
          fundingAccount: this.seller.dispensingAccounts[custodyId],
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          dispensingCustodyMint:
            this.dispensingCustodies[custodyId].mint.publicKey,
          dispensingCustody: this.dispensingCustodies[custodyId].tokenAccount,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
          rent: SYSVAR_RENT_PUBKEY,
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

  removeTokens = async (uiAmount: number, custodyId: number) => {
    try {
      await this.program.methods
        .removeTokens({
          amount: this.toTokenAmount(
            uiAmount,
            this.dispensingCustodies[custodyId].decimals
          ),
        })
        .accounts({
          owner: this.seller.wallet.publicKey,
          receivingAccount: this.seller.dispensingAccounts[custodyId],
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          dispensingCustody: this.dispensingCustodies[custodyId].tokenAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
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

  whitelistAdd = async (addresses: PublicKey[]) => {
    let bids = [];
    for (const address of addresses) {
      bids.push({
        isSigner: false,
        isWritable: true,
        pubkey: await this.getBidAddress(address),
      });
    }
    try {
      await this.program.methods
        .whitelistAdd({
          addresses: addresses,
        })
        .accounts({
          owner: this.seller.wallet.publicKey,
          auction: this.auction.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(bids)
        .signers([this.seller.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  whitelistRemove = async (addresses: PublicKey[]) => {
    let bids = [];
    for (const address of addresses) {
      bids.push({
        isSigner: false,
        isWritable: true,
        pubkey: await this.getBidAddress(address),
      });
    }
    try {
      await this.program.methods
        .whitelistRemove({})
        .accounts({
          owner: this.seller.wallet.publicKey,
          auction: this.auction.publicKey,
        })
        .remainingAccounts(bids)
        .signers([this.seller.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  withdrawFunds = async (amount: number, custody, receivingAccount) => {
    try {
      await this.program.methods
        .withdrawFunds({
          amount: new BN(amount),
        })
        .accounts({
          owner: this.seller.wallet.publicKey,
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          custody: custody.custody,
          custodyTokenAccount: custody.tokenAccount,
          sellerBalance: this.seller.balanceAccount,
          receivingAccount: receivingAccount,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
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

  getAuctionAmount = async (price: number) => {
    try {
      return await this.program.methods
        .getAuctionAmount({
          price: this.toTokenAmount(price, this.pricingCustody.decimals),
        })
        .accounts({
          user: this.provider.wallet.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
        })
        .view();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  getAuctionPrice = async (uiAmount: number) => {
    try {
      return await this.program.methods
        .getAuctionPrice({
          amount: this.toTokenAmount(uiAmount, this.pricingCustody.decimals),
        })
        .accounts({
          user: this.provider.wallet.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
        })
        .view();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  placeBidNoLookupTable = async (
    price: number,
    amount: number,
    bidType,
    user
  ) => {
    try {
      await this.program.methods
        .placeBid({
          price: this.toTokenAmount(price, this.pricingCustody.decimals),
          amount: new BN(amount),
          bidType: bidType,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount: user.paymentAccount,
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          sellerBalance: this.seller.balanceAccount,
          bid: await this.getBidAddress(user.wallet.publicKey),
          pricingCustody: this.pricingCustody.custody,
          pricingOracleAccount: this.pricingCustody.oracleAccount,
          paymentCustody: this.paymentCustody.custody,
          paymentOracleAccount: this.paymentCustody.oracleAccount,
          paymentTokenAccount: this.paymentCustody.tokenAccount,
          recentSlothashes: SYSVAR_SLOT_HASHES_PUBKEY,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts([
          ...user.receivingAccountMetas,
          ...this.dispensingAccountMetas,
        ])
        .signers([user.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  placeBid = async (price: number, amount: number, bidType, user) => {
    try {
      let ix = await this.program.methods
        .placeBid({
          price: this.toTokenAmount(price, this.pricingCustody.decimals),
          amount: new BN(amount),
          bidType: bidType,
        })
        .accounts({
          owner: user.wallet.publicKey,
          fundingAccount: user.paymentAccount,
          transferAuthority: this.authority.publicKey,
          launchpad: this.launchpad.publicKey,
          auction: this.auction.publicKey,
          sellerBalance: this.seller.balanceAccount,
          bid: await this.getBidAddress(user.wallet.publicKey),
          pricingCustody: this.pricingCustody.custody,
          pricingOracleAccount: this.pricingCustody.oracleAccount,
          paymentCustody: this.paymentCustody.custody,
          paymentOracleAccount: this.paymentCustody.oracleAccount,
          paymentTokenAccount: this.paymentCustody.tokenAccount,
          recentSlothashes: SYSVAR_SLOT_HASHES_PUBKEY,
          instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
          systemProgram: SystemProgram.programId,
          tokenProgram: spl.TOKEN_PROGRAM_ID,
        })
        .remainingAccounts([
          ...user.receivingAccountMetas,
          ...this.dispensingAccountMetas,
        ])
        .signers([user.wallet])
        .instruction();

      const lookupTableAccount = await this.provider.connection
        .getAddressLookupTable(this.lookupTable.publicKey)
        .then((res) => res.value);

      const latestBlockhash =
        await this.provider.connection.getLatestBlockhash();

      const messageV0 = new TransactionMessage({
        payerKey: user.wallet.publicKey,
        recentBlockhash: latestBlockhash.blockhash,
        instructions: [ix],
      }).compileToV0Message([lookupTableAccount]);

      const transactionV0 = new VersionedTransaction(messageV0);
      transactionV0.sign([user.wallet]);

      await this.provider.connection.confirmTransaction({
        signature: await this.provider.connection.sendTransaction(
          transactionV0
        ),
        blockhash: latestBlockhash.blockhash,
        lastValidBlockHeight: latestBlockhash.lastValidBlockHeight,
      });
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };

  cancelBid = async (user, initializer) => {
    try {
      await this.program.methods
        .cancelBid({})
        .accounts({
          initializer: initializer.wallet.publicKey,
          auction: this.auction.publicKey,
          bid: await this.getBidAddress(user.wallet.publicKey),
        })
        .signers([initializer.wallet])
        .rpc();
    } catch (err) {
      if (this.printErrors) {
        console.log(err);
      }
      throw err;
    }
  };
}
