import { SorobanClient } from "../soroban.client";
import {
    borrow,
    cleanSlenderEnvKeys,
    collatCoeff,
    debtTokenBalanceOf,
    debtTokenTotalSupply,
    deploy,
    deposit,
    init,
    mintUnderlyingTo,
    repay,
    sTokenBalanceOf,
    sTokenTotalSupply,
    sTokenUnderlyingBalanceOf,
    tokenBalanceOf,
    withdraw
} from "../pool.sut";
import {
    borrower1Keys,
    borrower2Keys,
    lender1Keys,
    lender2Keys,
    treasuryKeys
} from "../soroban.config";
import { assert, expect, use } from "chai";
import chaiAsPromised from 'chai-as-promised';
use(chaiAsPromised);

describe("LendingPool: Lenders get and borrowers pay interest when time passed", function () {
    let client: SorobanClient;
    let lender1Address: string;
    let borrower1Address: string;
    let lender2Address: string;
    let borrower2Address: string;
    let treasuryAddress: string;

    before(async function () {
        client = new SorobanClient();

        await cleanSlenderEnvKeys();
        await deploy();
        await init(client);

        lender1Address = lender1Keys.publicKey();
        lender2Address = lender2Keys.publicKey();
        borrower1Address = borrower1Keys.publicKey();
        borrower2Address = borrower2Keys.publicKey();
        treasuryAddress = treasuryKeys.publicKey();

        await client.registerAccount(lender1Address);
        await client.registerAccount(lender2Address);
        await client.registerAccount(borrower1Address);
        await client.registerAccount(borrower2Address);

        await mintUnderlyingTo(client, "XLM", lender1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "XRP", lender2Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower1Address, 100_000_000_000n);
        await mintUnderlyingTo(client, "USDC", borrower2Address, 100_000_000_000n);
    });

    it("Case 1: Lenders & borrowers deposit into pool", async function () {
        // Lender1 deposits 10_000_000_000 XLM
        await deposit(client, lender1Keys, "XLM", 10_000_000_000n);

        // Lender2 deposits 10_000_000_000 XRP
        await deposit(client, lender2Keys, "XRP", 10_000_000_000n);

        // Borrower1 deposits 10_000_000_000 USDC
        await deposit(client, borrower1Keys, "USDC", 20_000_000_000n);

        // Borrower2 deposits 10_000_000_000 USDC
        await deposit(client, borrower2Keys, "USDC", 20_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalance = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalance = await sTokenBalanceOf(client, "XRP", lender2Address);

        const borrower1UsdcBalance = await tokenBalanceOf(client, "USDC", borrower1Address);
        const borrower1SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower1Address);
        const borrower2UsdcBalance = await tokenBalanceOf(client, "USDC", borrower2Address);
        const borrower2SUsdcBalance = await sTokenBalanceOf(client, "USDC", borrower2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const sUsdcBalance = await sTokenUnderlyingBalanceOf(client, "USDC");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");
        const sUsdcSupply = await sTokenTotalSupply(client, "USDC");

        assert.equal(lender1XlmBalance, 90_000_000_000n);
        assert.equal(lender1SXlmBalance, 10_000_000_000n);
        assert.equal(lender2XrpBalance, 90_000_000_000n);
        assert.equal(lender2SXrpBalance, 10_000_000_000n);

        assert.equal(borrower1UsdcBalance, 80_000_000_000n);
        assert.equal(borrower1SUsdcBalance, 20_000_000_000n);
        assert.equal(borrower2UsdcBalance, 80_000_000_000n);
        assert.equal(borrower2SUsdcBalance, 20_000_000_000n);

        assert.equal(sXlmBalance, 10_000_000_000n);
        assert.equal(sXrpBalance, 10_000_000_000n);
        assert.equal(sUsdcBalance, 40_000_000_000n);

        assert.equal(sXlmSupply, 10_000_000_000n);
        assert.equal(sXrpSupply, 10_000_000_000n);
        assert.equal(sUsdcSupply, 40_000_000_000n);
    });

    it("Case 2: Borrowers borrow assets from pool with max utilization", async function () {
        // Borrower1 borrows 10_000_000_000 XLM
        await borrow(client, borrower1Keys, "XLM", 9_000_000_000n);

        // Borrower2 borrows 10_000_000_000 XRP
        await borrow(client, borrower2Keys, "XRP", 9_000_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower2XrpBalance = await tokenBalanceOf(client, "XRP", borrower2Address);

        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const borrower2DXrpBalance = await debtTokenBalanceOf(client, "XRP", borrower2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");
        const dXrpSupply = await debtTokenTotalSupply(client, "XRP");

        assert.equal(borrower1XlmBalance, 9_000_000_000n);
        assert.equal(borrower2XrpBalance, 9_000_000_000n);

        assert.equal(borrower1DXlmBalance, 9_000_000_000n);
        assert.equal(borrower2DXrpBalance, 9_000_000_000n);

        assert.equal(sXlmBalance, 1_000_000_000n);
        assert.equal(sXrpBalance, 1_000_000_000n);

        assert.equal(dXlmSupply, 9_000_000_000n);
        assert.equal(dXrpSupply, 9_000_000_000n);
    });

    it("Case 3: Borrowers try to borrow more when max utilization exceeded", async function () {
        // Borrower1 borrows 1_000_000_000 XLM
        await expect(borrow(client, borrower1Keys, "XLM", 1_000_000_000n)).to.eventually.rejected;

        // Borrower2 borrows 1_000_000_000 XRP
        await expect(borrow(client, borrower2Keys, "XRP", 1_000_000_000n)).to.eventually.rejected;

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const borrower2XrpBalance = await tokenBalanceOf(client, "XRP", borrower2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        assert.equal(borrower1XlmBalance, 9_000_000_000n);
        assert.equal(borrower2XrpBalance, 9_000_000_000n);

        assert.equal(sXlmBalance, 1_000_000_000n);
        assert.equal(sXrpBalance, 1_000_000_000n);
    });

    it("Case 4: Collateral coefficient should be increased as time goes", async function () {
        const xlmCollatCoeff = await collatCoeff(client, "XLM");
        const xrpCollatCoeff = await collatCoeff(client, "XRP");
        const usdcCollatCoeff = await collatCoeff(client, "USDC");

        assert(xlmCollatCoeff > 1_000_000_000n);
        assert(xrpCollatCoeff > 1_000_000_000n);
        assert(usdcCollatCoeff == 1_000_000_000n);
    });

    it("Case 5: Lenders withdraw to make utilization ~ 1", async function () {
        // Lender1 withdraws 1_000_000_000 XLM
        await withdraw(client, lender1Keys, "XLM", 1_000_000_000n);

        // Lender2 withdraws 1_000_000_000 XRP
        await withdraw(client, lender2Keys, "XRP", 1_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalance = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalance = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalance, 91_000_000_000n);
        assert(lender1SXlmBalance < 9_001_000_000n
            && lender1SXlmBalance > 9_000_000_000n);
        assert.equal(lender2XrpBalance, 91_000_000_000n);
        assert(lender2SXrpBalance < 9_001_000_000n
            && lender2SXrpBalance > 9_000_000_000n);

        assert.equal(sXlmBalance, 0n);
        assert.equal(sXrpBalance, 0n);

        assert.equal(sXlmSupply, lender1SXlmBalance);
        assert.equal(sXrpSupply, lender2SXrpBalance);
    });

    it("Case 6: Lenders try to make overwithdraw when utilization ~ 1", async function () {
        // Lender1 withdraws 1_000_000_000 XLM
        await expect(withdraw(client, lender1Keys, "XLM", 1_000_000_000n)).to.eventually.rejected;

        // Lender2 withdraws 1_000_000_000 XRP
        await expect(withdraw(client, lender2Keys, "XRP", 1_000_000_000n)).to.eventually.rejected;

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalance = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalance = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalance, 91_000_000_000n);
        assert.equal(lender2XrpBalance, 91_000_000_000n);

        assert.equal(sXlmBalance, 0n);
        assert.equal(sXrpBalance, 0n);

        assert.equal(sXlmSupply, lender1SXlmBalance);
        assert.equal(sXrpSupply, lender2SXrpBalance);
    });

    it("Case 7: Borrower1 makes partial repay", async function () {
        // Borrower1 repays 1_000_000_000 XLM
        await repay(client, borrower1Keys, "XLM", 1_000_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const treasuryXlmBalance = await tokenBalanceOf(client, "XLM", treasuryAddress);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");

        assert.equal(borrower1XlmBalance, 8_000_000_000n);
        assert(treasuryXlmBalance > 0 && treasuryXlmBalance < 100_000n);
        assert(borrower1DXlmBalance > 8_000_000_000n
            && borrower1DXlmBalance < 8_001_000_000n);
        assert.equal(sXlmBalance + treasuryXlmBalance, 1_000_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);
    });

    it("Case 8: Borrower1 makes full repay", async function () {
        // Borrower1 repays 9_000_000_000 XLM
        await mintUnderlyingTo(client, "XLM", borrower1Address, 1_000_000_000n);
        await repay(client, borrower1Keys, "XLM", 9_000_000_000n);

        const borrower1XlmBalance = await tokenBalanceOf(client, "XLM", borrower1Address);
        const treasuryXlmBalance = await tokenBalanceOf(client, "XLM", treasuryAddress);
        const borrower1DXlmBalance = await debtTokenBalanceOf(client, "XLM", borrower1Address);
        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const dXlmSupply = await debtTokenTotalSupply(client, "XLM");

        assert(borrower1XlmBalance < 1_000_000_000n
            && borrower1XlmBalance > 999_000_000n);
        assert(treasuryXlmBalance > 0 && treasuryXlmBalance < 100_000n);
        assert.equal(borrower1DXlmBalance, 0n);
        assert(sXlmBalance + treasuryXlmBalance > 9_000_000_000n
            && sXlmBalance + treasuryXlmBalance < 9_001_000_000n);
        assert.equal(dXlmSupply, borrower1DXlmBalance);
    });

    it("Case 9: Borrower2 makes full repay", async function () {
        // Borrower2 repays 10_000_000_000 XRP
        await mintUnderlyingTo(client, "XRP", borrower2Address, 1_000_000_000n);
        await repay(client, borrower2Keys, "XRP", 10_000_000_000n);

        const borrower2XrpBalance = await tokenBalanceOf(client, "XRP", borrower2Address);
        const treasuryXrpBalance = await tokenBalanceOf(client, "XRP", treasuryAddress);
        const borrower2DXrpBalance = await debtTokenBalanceOf(client, "XRP", borrower2Address);
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");
        const dXrpSupply = await debtTokenTotalSupply(client, "XRP");

        assert(borrower2XrpBalance < 1_000_000_000n
            && borrower2XrpBalance > 999_000_000n);
        assert(treasuryXrpBalance > 0 && treasuryXrpBalance < 100_000n);
        assert.equal(borrower2DXrpBalance, 0n);
        assert(sXrpBalance + treasuryXrpBalance > 9_000_000_000n
            && sXrpBalance + treasuryXrpBalance < 9_001_000_000n);
        assert.equal(dXrpSupply, borrower2DXrpBalance);
    });

    it("Case 10: Lenders make partial withdraw", async function () {
        // Lender1 withdraws 1_000_000_000 XLM
        await withdraw(client, lender1Keys, "XLM", 1_000_000_000n);

        // Lender2 withdraws 1_000_000_000 XRP
        await withdraw(client, lender2Keys, "XRP", 1_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalance = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalance = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert.equal(lender1XlmBalance, 92_000_000_000n);
        assert(lender1SXlmBalance < 8_001_000_000n
            && lender1SXlmBalance > 8_000_000_000n);
        assert.equal(lender2XrpBalance, 92_000_000_000n);
        assert(lender2SXrpBalance < 8_001_000_000n
            && lender2SXrpBalance > 8_000_000_000n);

        assert(sXlmBalance > 8_000_000_000n
            && sXlmBalance < 8_001_000_000n);
        assert(sXrpBalance > 8_000_000_000n
            && sXrpBalance < 8_001_000_000n);

        assert.equal(sXlmSupply, lender1SXlmBalance);
        assert.equal(sXrpSupply, lender2SXrpBalance);
    });

    it("Case 11: Lenders make full withdraw", async function () {
        // Lender1 withdraws 10_000_000_000 XLM
        await withdraw(client, lender1Keys, "XLM", 10_000_000_000n);

        // Lender2 withdraws 10_000_000_000 XRP
        await withdraw(client, lender2Keys, "XRP", 10_000_000_000n);

        const lender1XlmBalance = await tokenBalanceOf(client, "XLM", lender1Address);
        const lender1SXlmBalance = await sTokenBalanceOf(client, "XLM", lender1Address);
        const lender2XrpBalance = await tokenBalanceOf(client, "XRP", lender2Address);
        const lender2SXrpBalance = await sTokenBalanceOf(client, "XRP", lender2Address);

        const sXlmBalance = await sTokenUnderlyingBalanceOf(client, "XLM");
        const sXrpBalance = await sTokenUnderlyingBalanceOf(client, "XRP");

        const sXlmSupply = await sTokenTotalSupply(client, "XLM");
        const sXrpSupply = await sTokenTotalSupply(client, "XRP");

        assert(lender1XlmBalance > 100_000_000_000n
            && lender1XlmBalance < 100_001_000_000n);
        assert.equal(lender1SXlmBalance, 0n);
        assert(lender2XrpBalance > 100_000_000_000n
            && lender2XrpBalance < 100_001_000_000n);
        assert.equal(lender2SXrpBalance, 0n);

        assert(sXlmBalance < 1_000n);
        assert(sXrpBalance < 1_000n);

        assert.equal(sXlmSupply, lender1SXlmBalance);
        assert.equal(sXrpSupply, lender2SXrpBalance);
    });
});