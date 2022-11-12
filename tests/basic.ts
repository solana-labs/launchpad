import * as anchor from "@project-serum/anchor";
import { LaunchpadTester } from "./launchpad_tester";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import * as spl from "@solana/spl-token";
import { expect, assert } from "chai";
import { BN } from "bn.js";

describe("launchpad", () => {
  let lpd = new LaunchpadTester();
  lpd.printErrors = true;
  let launchpadExpected;
  let multisigExpected;
  let auctionExpected;
  let auctionParams;

  it("init", async () => {
    await lpd.initFixture();
    await lpd.init();

    let err = await lpd.ensureFails(lpd.init());
    assert(err.logs[3].includes("already in use"));

    launchpadExpected = {
      permissions: {
        allowNewAuctions: true,
        allowAuctionUpdates: true,
        allowAuctionRefills: true,
        allowAuctionPullouts: true,
        allowNewBids: true,
        allowWithdrawals: true,
      },
      fees: {
        newAuction: new BN(100),
        auctionUpdate: new BN(100),
        invalidBid: { numerator: "1", denominator: "100" },
        trade: { numerator: "1", denominator: "100" },
      },
      collectedFees: {
        newAuctionSol: "0",
        auctionUpdateSol: "0",
        invalidBidUsdc: "0",
        tradeUsdc: "0",
      },
      transferAuthorityBump: lpd.authority.bump,
      launchpadBump: lpd.multisig.bump,
    };

    multisigExpected = {
      numSigners: 2,
      numSigned: 0,
      minSignatures: 2,
      instructionAccountsLen: 0,
      instructionDataLen: 0,
      instructionHash: new anchor.BN(0),
      signers: [
        lpd.admins[0].publicKey,
        lpd.admins[1].publicKey,
        PublicKey.default,
        PublicKey.default,
        PublicKey.default,
        PublicKey.default,
      ],
      signed: [false, false, false, false, false, false],
      bump: lpd.multisig.bump,
    };

    let multisig = await lpd.program.account.multisig.fetch(
      lpd.multisig.publicKey
    );
    expect(JSON.stringify(multisig)).to.equal(JSON.stringify(multisigExpected));

    let launchpad = await lpd.program.account.launchpad.fetch(
      lpd.launchpad.publicKey
    );
    expect(JSON.stringify(launchpad)).to.equal(
      JSON.stringify(launchpadExpected)
    );
  });

  it("setAdminSigners", async () => {
    await lpd.setAdminSigners(1);

    let multisig = await lpd.program.account.multisig.fetch(
      lpd.multisig.publicKey
    );
    multisigExpected.minSignatures = 1;
    expect(JSON.stringify(multisig)).to.equal(JSON.stringify(multisigExpected));
  });

  it("setFees", async () => {
    launchpadExpected.fees = {
      newAuction: new BN(10000),
      auctionUpdate: new BN(100000),
      invalidBid: { numerator: new BN(1), denominator: new BN(1000) },
      trade: { numerator: new BN(1), denominator: new BN(1000) },
    };
    await lpd.setFees(launchpadExpected.fees);

    let launchpad = await lpd.program.account.launchpad.fetch(
      lpd.launchpad.publicKey
    );
    expect(JSON.stringify(launchpad)).to.equal(
      JSON.stringify(launchpadExpected)
    );
  });

  it("setPermissions", async () => {
    launchpadExpected.permissions = {
      allowNewAuctions: false,
      allowAuctionUpdates: false,
      allowAuctionRefills: false,
      allowAuctionPullouts: false,
      allowNewBids: false,
      allowWithdrawals: false,
    };
    await lpd.setPermissions(launchpadExpected.permissions);

    let launchpad = await lpd.program.account.launchpad.fetch(
      lpd.launchpad.publicKey
    );
    expect(JSON.stringify(launchpad)).to.equal(
      JSON.stringify(launchpadExpected)
    );
  });

  it("initCustodies", async () => {
    let config = {
      maxOraclePriceError: 1,
      maxOraclePriceAgeSec: 60,
      oracleType: { test: {} },
      oracleAccount: lpd.pricingCustody.oracleAccount,
    };
    await lpd.initCustody(config, lpd.pricingCustody);

    config.oracleAccount = lpd.paymentCustody.oracleAccount;
    await lpd.initCustody(config, lpd.paymentCustody);

    let custody = await lpd.program.account.custody.fetch(
      lpd.pricingCustody.custody
    );
    let custodyExpected = {
      tokenAccount: lpd.pricingCustody.tokenAccount,
      collectedFees: new BN(0),
      mint: lpd.pricingCustody.mint.publicKey,
      decimals: lpd.pricingCustody.decimals,
      maxOraclePriceError: config.maxOraclePriceError,
      maxOraclePriceAgeSec: config.maxOraclePriceAgeSec,
      oracleType: config.oracleType,
      oracleAccount: lpd.pricingCustody.oracleAccount,
      bump: custody.bump,
    };
    expect(JSON.stringify(custody)).to.equal(JSON.stringify(custodyExpected));
  });

  it("setOracleConfig", async () => {
    let config = {
      maxOraclePriceError: 123,
      maxOraclePriceAgeSec: 900,
      oracleType: { test: {} },
      oracleAccount: lpd.paymentCustody.oracleAccount,
    };
    let custodyExpected = await lpd.program.account.custody.fetch(
      lpd.paymentCustody.custody
    );
    custodyExpected.maxOraclePriceError = config.maxOraclePriceError;
    custodyExpected.maxOraclePriceAgeSec = config.maxOraclePriceAgeSec;
    custodyExpected.oracleType = config.oracleType;
    custodyExpected.oracleAccount = config.oracleAccount;

    await lpd.setOracleConfig(config, lpd.paymentCustody);

    let custody = await lpd.program.account.custody.fetch(
      lpd.paymentCustody.custody
    );
    expect(JSON.stringify(custody)).to.equal(JSON.stringify(custodyExpected));
  });

  it("initAuction", async () => {
    auctionParams = {
      enabled: true,
      updatable: true,
      fixedAmount: false,
      common: {
        name: "test auction",
        description: "test only",
        aboutSeller: "Tester",
        sellerLink: "solana.com",
        startTime: new BN(222),
        endTime: new BN(2222),
        presaleStartTime: new BN(111),
        presaleEndTime: new BN(222),
        fillLimitRegAddress: new BN(10),
        fillLimitWlAddress: new BN(20),
        orderLimitRegAddress: new BN(5),
        orderLimitWlAddress: new BN(10),
      },
      payment: {
        acceptSol: true,
        acceptUsdc: true,
        acceptOtherTokens: true,
      },
      pricing: {
        custody: lpd.pricingCustody.custody,
        pricingModel: { dynamicDutchAuction: {} },
        startPrice: new BN(100),
        maxPrice: new BN(200),
        minPrice: new BN(90),
        repriceDelay: new BN(5),
        repriceCoef: new BN(1000),
        repriceFunction: { linear: {} },
        amountFunction: { fixed: {} },
        amountPerLevel: new BN(200),
        tickSize: new BN(2),
        unitSize: lpd.toTokenAmount(1, lpd.dispensingCustodies[0].decimals),
      },
      tokenRatios: [new BN(1), new BN(2)],
    };

    let err = await lpd.ensureFails(lpd.initAuction(auctionParams));
    assert(err.error.errorCode.code === "NewAuctionsNotAllowed");

    launchpadExpected.permissions = {
      allowNewAuctions: true,
      allowAuctionUpdates: true,
      allowAuctionRefills: true,
      allowAuctionPullouts: true,
      allowNewBids: true,
      allowWithdrawals: true,
    };
    await lpd.setPermissions(launchpadExpected.permissions);

    await lpd.initAuction(auctionParams);

    let auction = await lpd.program.account.auction.fetch(
      lpd.auction.publicKey
    );
    auctionExpected = {
      owner: lpd.seller.wallet.publicKey,
      enabled: true,
      updatable: true,
      fixedAmount: false,
      common: auctionParams.common,
      payment: auctionParams.payment,
      pricing: auctionParams.pricing,
      stats: {
        firstTradeTime: "0",
        lastTradeTime: "0",
        lastAmount: "0",
        lastPrice: "0",
        wlBidders: {
          fillsVolume: "0",
          weightedFillsSum: "0",
          minFillPrice: "18446744073709551615",
          maxFillPrice: "0",
          numTrades: "0",
        },
        regBidders: {
          fillsVolume: "0",
          weightedFillsSum: "0",
          minFillPrice: "18446744073709551615",
          maxFillPrice: "0",
          numTrades: "0",
        },
      },
      tokens: [
        { ratio: "1", account: lpd.dispensingCustodies[0].tokenAccount },
        { ratio: "2", account: lpd.dispensingCustodies[1].tokenAccount },
        { ratio: "0", account: "11111111111111111111111111111111" },
        { ratio: "0", account: "11111111111111111111111111111111" },
      ],
      numTokens: 2,
      creationTime: "0",
      updateTime: "0",
      bump: auction.bump,
    };
    expect(JSON.stringify(auction)).to.equal(JSON.stringify(auctionExpected));
  });

  it("updateAuction", async () => {
    auctionParams.common.description = "updated";
    let params = {
      common: auctionParams.common,
      payment: auctionParams.payment,
      pricing: auctionParams.pricing,
      tokenRatios: auctionParams.tokenRatios,
    };
    await lpd.updateAuction(params);

    let auction = await lpd.program.account.auction.fetch(
      lpd.auction.publicKey
    );
    auctionExpected.common.description = "updated";
    expect(JSON.stringify(auction)).to.equal(JSON.stringify(auctionExpected));
  });

  it("disableAuction", async () => {
    await lpd.disableAuction();
    let auction = await lpd.program.account.auction.fetch(
      lpd.auction.publicKey
    );
    auctionExpected.enabled = false;
    expect(JSON.stringify(auction)).to.equal(JSON.stringify(auctionExpected));
  });

  it("enableAuction", async () => {
    await lpd.enableAuction();
    let auction = await lpd.program.account.auction.fetch(
      lpd.auction.publicKey
    );
    auctionExpected.enabled = true;
    expect(JSON.stringify(auction)).to.equal(JSON.stringify(auctionExpected));
  });

  it("addTokens", async () => {
    for (let i = 0; i < lpd.seller.dispensingAccounts.length; ++i) {
      let initialSourceBalance = await lpd.getBalance(
        lpd.seller.dispensingAccounts[i]
      );
      let initialDestinationBalance = await lpd.getBalance(
        lpd.dispensingCustodies[i].tokenAccount
      );
      await lpd.addTokens(200, i);
      let sourceBalance = await lpd.getBalance(
        lpd.seller.dispensingAccounts[i]
      );
      let destinationBalance = await lpd.getBalance(
        lpd.dispensingCustodies[i].tokenAccount
      );
      expect(initialSourceBalance - sourceBalance).to.equal(
        200 * 10 ** lpd.dispensingCustodies[i].decimals
      );
      expect(destinationBalance - initialDestinationBalance).to.equal(
        200 * 10 ** lpd.dispensingCustodies[i].decimals
      );
    }
  });

  it("removeTokens", async () => {
    let initialSourceBalance = await lpd.getBalance(
      lpd.seller.dispensingAccounts[0]
    );
    let initialDestinationBalance = await lpd.getBalance(
      lpd.dispensingCustodies[0].tokenAccount
    );
    await lpd.removeTokens(50, 0);
    let sourceBalance = await lpd.getBalance(lpd.seller.dispensingAccounts[0]);
    let destinationBalance = await lpd.getBalance(
      lpd.dispensingCustodies[0].tokenAccount
    );
    expect(sourceBalance - initialSourceBalance).to.equal(
      50 * 10 ** lpd.dispensingCustodies[0].decimals
    );
    expect(initialDestinationBalance - destinationBalance).to.equal(
      50 * 10 ** lpd.dispensingCustodies[0].decimals
    );
  });

  it("setTestOraclePrice", async () => {
    await lpd.setTestOraclePrice(123, lpd.paymentCustody);
    await lpd.setTestOraclePrice(200, lpd.pricingCustody);

    let oracle = await lpd.program.account.testOracle.fetch(
      lpd.paymentCustody.oracleAccount
    );
    let oracleExpected = {
      price: new BN(123000),
      expo: -3,
      conf: new BN(0),
      publishTime: oracle.publishTime,
    };
    expect(JSON.stringify(oracle)).to.equal(JSON.stringify(oracleExpected));
  });

  it("setTestTime", async () => {
    await lpd.setTestTime(111);

    let auction = await lpd.program.account.auction.fetch(
      lpd.auction.publicKey
    );
    expect(JSON.stringify(auction.creationTime)).to.equal(
      JSON.stringify(new BN(111))
    );
  });

  it("whitelistAdd", async () => {
    await lpd.whitelistAdd([
      lpd.users[0].wallet.publicKey,
      lpd.users[1].wallet.publicKey,
    ]);

    let bid = await lpd.program.account.bid.fetch(
      await lpd.getBidAddress(lpd.users[1].wallet.publicKey)
    );
    let bidExpected = {
      owner: lpd.users[1].wallet.publicKey,
      auction: lpd.auction.publicKey,
      whitelisted: true,
      sellerInitialized: true,
      bidTime: new BN(0),
      bidPrice: new BN(0),
      bidAmount: new BN(0),
      bidType: { ioc: {} },
      filled: new BN(0),
      fillTime: new BN(0),
      fillPrice: new BN(0),
      fillAmount: new BN(0),
      bump: bid.bump,
    };
    expect(JSON.stringify(bid)).to.equal(JSON.stringify(bidExpected));
  });

  it("whitelistRemove", async () => {
    await lpd.whitelistRemove([lpd.users[1].wallet.publicKey]);

    let bid = await lpd.program.account.bid.fetch(
      await lpd.getBidAddress(lpd.users[1].wallet.publicKey)
    );
    let bidExpected = {
      owner: lpd.users[1].wallet.publicKey,
      auction: lpd.auction.publicKey,
      whitelisted: false,
      sellerInitialized: true,
      bidTime: new BN(0),
      bidPrice: new BN(0),
      bidAmount: new BN(0),
      bidType: { ioc: {} },
      filled: new BN(0),
      fillTime: new BN(0),
      fillPrice: new BN(0),
      fillAmount: new BN(0),
      bump: bid.bump,
    };
    expect(JSON.stringify(bid)).to.equal(JSON.stringify(bidExpected));
  });

  it("getAuctionAmount", async () => {
    let amount = await lpd.getAuctionAmount(100);
    console.log("AMOUNT:", amount);
    //expect(amount).to.equal(100);
  });

  it("getAuctionPrice", async () => {
    let price = await lpd.getAuctionPrice(100);
    console.log("PRICE:", price);
    //expect(price).to.equal(100);
  });

  it("placeBid", async () => {
    let user = lpd.users[0];

    // record initial balances
    let initialBalancePayment = await lpd.getBalance(user.paymentAccount);
    let initialBalancesReceiving = [];
    for (const meta of user.receivingAccountMetas) {
      initialBalancesReceiving.push(await lpd.getBalance(meta.pubkey));
    }
    let initialReceivingSum = initialBalancesReceiving.reduce(
      (a, b) => a + b,
      0
    );

    // place the bid
    let bidAmount = 1;
    let bidPrice = 100;
    let bidType = { ioc: {} };
    let availAmount = (await lpd.getAuctionAmount(bidPrice)).toNumber();
    await lpd.placeBid(bidPrice, bidAmount, bidType, user);

    // check updated balances
    let balancePayment = await lpd.getBalance(user.paymentAccount);
    /*expect(balancePayment).to.equal(
      initialBalancePayment -
        lpd.toTokenAmount(bidPrice, lpd.paymentCustody.decimals).toNumber()
    );*/
    //initialBalancePayment - pricePerToken * fillAmount;

    let balancesReceiving = [];
    for (const meta of user.receivingAccountMetas) {
      balancesReceiving.push(await lpd.getBalance(meta.pubkey));
    }
    let expectedFillAmount =
      Math.min(bidAmount, availAmount) *
      auctionParams.pricing.unitSize.toNumber();
    let receivingSum = balancesReceiving.reduce((a, b) => a + b, 0);
    expect(receivingSum).to.equal(initialReceivingSum + expectedFillAmount);

    // check bid account
    let bid = await lpd.program.account.bid.fetch(
      await lpd.getBidAddress(user.wallet.publicKey)
    );
    let auction = await lpd.program.account.auction.fetch(
      lpd.auction.publicKey
    );
    //expect(auction).to.equal(auctionExpected);
    let bidExpected = {
      owner: user.wallet.publicKey,
      auction: lpd.auction.publicKey,
      whitelisted: true,
      sellerInitialized: true,
      bidTime: auction.creationTime,
      bidPrice: lpd.toTokenAmount(bidPrice, lpd.pricingCustody.decimals),
      bidAmount: new BN(bidAmount),
      bidType: bidType,
      filled: new BN(bidAmount),
      fillTime: auction.creationTime,
      fillPrice: new BN(100),
      fillAmount: new BN(bidAmount),
      bump: bid.bump,
    };
    expect(JSON.stringify(bid)).to.equal(JSON.stringify(bidExpected));

    // check seller's balance account
    let sellerBalance = await lpd.program.account.sellerBalance.fetch(
      lpd.seller.balanceAccount
    );
    console.log(JSON.stringify(sellerBalance));
  });

  it("cancelBid", async () => {
    await lpd.setTestTime(22222);

    let user = lpd.users[0];
    let initializer = lpd.seller;
    let initialBalanceSol = await lpd.getSolBalance(
      initializer.wallet.publicKey
    );

    await lpd.cancelBid(user, initializer);

    let balanceSol = await lpd.getSolBalance(initializer.wallet.publicKey);
    expect(initialBalanceSol).to.lessThan(balanceSol);

    await lpd.ensureFails(
      lpd.program.account.bid.fetch(
        await lpd.getBidAddress(initializer.wallet.publicKey)
      ),
      "Fetch Bid should've been failed"
    );
  });

  it("withdrawFees", async () => {
    let initialBalanceTokens = await lpd.getBalance(lpd.feesAccount);
    let withdrawAmountTokens = (
      await lpd.program.account.custody.fetch(lpd.paymentCustody.custody)
    ).collectedFees.toNumber();
    expect(withdrawAmountTokens).to.greaterThan(0);

    let initialBalanceSol = await lpd.getSolBalance(lpd.admins[0].publicKey);
    let withdrawAmountSol = await lpd.getSolBalance(lpd.authority.publicKey);
    expect(withdrawAmountSol).to.greaterThan(0);

    await lpd.withdrawFees(
      withdrawAmountTokens,
      withdrawAmountSol,
      lpd.paymentCustody,
      lpd.feesAccount,
      lpd.admins[0].publicKey
    );

    let balanceTokens = await lpd.getBalance(lpd.feesAccount);
    expect(balanceTokens).to.equal(initialBalanceTokens + withdrawAmountTokens);

    let balanceSol = await lpd.getSolBalance(lpd.admins[0].publicKey);
    expect(balanceSol).to.equal(initialBalanceSol + withdrawAmountSol);
  });

  it("withdrawFunds", async () => {
    let initialBalance = await lpd.getBalance(lpd.seller.paymentAccount);
    let withdrawAmount = (
      await lpd.program.account.sellerBalance.fetch(lpd.seller.balanceAccount)
    ).balance.toNumber();
    expect(withdrawAmount).to.greaterThan(0);

    await lpd.withdrawFunds(
      withdrawAmount,
      lpd.paymentCustody,
      lpd.seller.paymentAccount
    );

    let balance = await lpd.getBalance(lpd.seller.paymentAccount);
    expect(balance).to.equal(initialBalance + withdrawAmount);
  });

  it("deleteAuction", async () => {
    for (let i = 0; i < lpd.dispensingCustodies.length; ++i) {
      await lpd.removeTokens(
        lpd.toUiAmount(
          await lpd.getBalance(lpd.dispensingCustodies[i].tokenAccount),
          lpd.dispensingCustodies[i].decimals
        ),
        i
      );
    }
    await lpd.deleteAuction();
    await lpd.ensureFails(
      lpd.program.account.auction.fetch(lpd.auction.publicKey),
      "Fetch Auction should've been failed"
    );
    await lpd.ensureFails(
      lpd.getTokenAccount(lpd.dispensingCustodies[0].tokenAccount),
      "Get dispensing token account should've been failed"
    );
  });
});
