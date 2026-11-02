import assert from "node:assert/strict";
import { test } from "node:test";

import { close, engine, scenario } from "./setup.mjs";

const fl = await engine();
const { portfolio, market } = await scenario(fl);

test("the scenario's books, values and performance", () => {
  assert.deepEqual(fl.validatePortfolio(portfolio), []);
  const books = new fl.Books(portfolio, market);
  assert.equal(books.baseCurrency, "USD");
  assert.equal(books.defaultDates().length, 44);
  const valuation = books.valuation("2026-10-30");
  assert.ok(Number(valuation.total_base) > 0);
  const perf = books.performance();
  assert.ok(perf.returns instanceof Float64Array);
  assert.equal(perf.returns.length, 43);
  const total = perf.contributions().reduce((sum, c) => sum + c.contribution, 0);
  close(total, perf.timeWeighted(), 1e-12, "contributions");
  assert.equal(perf.period("MTD").start, "2026-09-30");
  assert.equal(perf.period("1Y"), undefined);
  const gains = books.realisedGains();
  assert.equal(gains.length, 1);
  assert.equal(gains[0].instrument, "SPY");
});

test("rebalancing proposals apply to the books", () => {
  const books = new fl.Books(portfolio, market);
  const targets = [
    [{ holding: "instrument", id: "SPY" }, 0.4],
    [{ holding: "instrument", id: "BOND" }, 0.4],
    [{ holding: "instrument", id: "SAP" }, 0.15],
  ];
  const plan = books.rebalance("main", targets, { lot_method: "highest_cost" });
  assert.equal(plan.date, "2026-10-30");
  const transactions = fl.rebalanceTransactions(plan);
  const after = new fl.Books({ ...portfolio, transactions: [...portfolio.transactions, ...transactions] }, market);
  const main = after.valuation(plan.date).positions.filter((p) => p.account === "main");
  const spy = main.find((p) => p.instrument === "SPY");
  close(Number(spy.market_value_base) / Number(plan.value_base), 0.4, 1e-6, "SPY weight");
});

test("rebalancing with fees, tax rates and several accounts", () => {
  const books = new fl.Books(portfolio, market);
  const targets = [
    [{ holding: "instrument", id: "SPY" }, 0.2],
    [{ holding: "instrument", id: "BOND" }, 0.5],
  ];
  const plan = books.rebalance("main", targets, {
    fees: { rate: "0.001", minimum: "5" },
    tax: { short_term: "0.4", long_term: "0.2" },
  });
  assert.ok(plan.trades.every((t) => Number(t.fees) >= 5));
  assert.ok(plan.tax_base !== null && plan.tax_base !== undefined);
  const combined = books.rebalanceAccounts(["isa", "main"], targets);
  const spy = combined.weights_after.find(([k]) => k.holding === "instrument" && k.id === "SPY");
  close(spy[1], 0.2, 1e-6, "combined SPY weight");
  books.free();
});

test("documents round trip", () => {
  const text = fl.writeDocument(portfolio, market.toDocument());
  const document = fl.readDocument(text);
  assert.deepEqual(document.portfolio, portfolio);
  assert.deepEqual(document.market.toDocument(), market.toDocument());
  assert.throws(() => fl.readDocument('{"format": "finalytics.portfolio", "format_version": 2, "portfolio": {}}'), {
    name: "ValidationError",
  });
});
