import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, scenario } from "./setup.mjs";

const fl = await engine();
const { portfolio, market } = await scenario(fl);

test("the scenario's books, values and performance", () => {
  assert.deepEqual(fl.validatePortfolio(portfolio), []);
  const books = new fl.Portfolio(portfolio, market);
  assert.equal(books.baseCurrency, "USD");
  assert.equal(books.dates().length, 44);
  const valuation = books.valuation("2026-10-30");
  assert.ok(Number(valuation.total_base) > 0);
  const perf = books.performance();
  assert.ok(perf.returns instanceof Float64Array);
  assert.equal(perf.returns.length, 43);
  assert.equal(perf.dates.length, 44);
  assert.deepEqual(perf.dates, books.dates());
  const total = perf.contributions().reduce((sum, c) => sum + c.contribution, 0);
  close(total, perf.timeWeighted(), 1e-12, "contributions");
  assert.equal(perf.period("MTD").start, "2026-09-30");
  assert.equal(perf.period("1Y"), undefined);
  const gains = books.realisedGains();
  assert.equal(gains.length, 1);
  assert.equal(gains[0].instrument, "SPY");
});

test("rebalancing proposals apply to the books", () => {
  const books = new fl.Portfolio(portfolio, market);
  const targets = { SPY: 0.4, BOND: 0.4, SAP: 0.15 };
  const proposal = books.rebalance(targets, { accounts: ["main"], lot_method: "highest_cost" });
  const [plan] = proposal.results.accounts;
  assert.equal(plan.date, "2026-10-30");
  const transactions = fl.rebalanceTransactions(plan);
  const after = new fl.Portfolio({ ...portfolio, transactions: [...portfolio.transactions, ...transactions] }, market);
  const main = after.valuation(plan.date).positions.filter((p) => p.account === "main");
  const spy = main.find((p) => p.instrument === "SPY");
  close(Number(spy.market_value_base) / Number(plan.value_base), 0.4, 1e-6, "SPY weight");
  assert.equal(books.withRebalance(proposal).valuation().total_base, after.valuation().total_base);
});

test("rebalancing with fees, tax rates and several accounts", () => {
  const books = new fl.Portfolio(portfolio, market);
  const targets = { SPY: 0.2, BOND: 0.5 };
  const [plan] = books.rebalance(targets, {
    accounts: ["main"],
    fees: { rate: "0.001", minimum: "5" },
    tax: { short_term: "0.4", long_term: "0.2" },
  }).results.accounts;
  assert.ok(plan.trades.every((t) => Number(t.fees_local) >= 5));
  assert.ok(plan.tax_base !== null && plan.tax_base !== undefined);
  const combined = books.rebalance(targets, { accounts: ["isa", "main"] });
  close(combined.results.weights_after.SPY, 0.2, 1e-6, "combined SPY weight");
  books.free();
});

test("documents round trip", () => {
  const text = fl.writeDocument(portfolio, market.toDocument());
  const document = fl.readDocument(text);
  assert.deepEqual(document.portfolio, portfolio);
  assert.deepEqual(document.market.toDocument(), market.toDocument());
  // The market data that readDocument returns writes back as it is.
  assert.equal(fl.writeDocument(document.portfolio, document.market), text);
  assert.throws(() => fl.readDocument('{"format": "finalytics.portfolio", "format_version": 2, "portfolio": {}}'), {
    name: "ValidationError",
  });
});
