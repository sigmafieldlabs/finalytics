import { datasets } from "finalytics";

const portfolio = datasets.portfolio();
const performance = portfolio.performance();
const market = portfolio.market.toDocument();
console.log("Benchmark column:", datasets.reportOptions().benchmark);
const excess = performance.excessReturn(
  portfolio.market.priceDates,
  Float64Array.from(market.prices.BENCH.values.map(Number)), "ITD",
);
console.log("Arithmetic:", excess.arithmetic.toFixed(6));
console.log("Geometric:", excess.geometric.toFixed(6));
