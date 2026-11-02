// Compiled with `tsc -p bindings/wasm/test` to check that the published declarations are usable
// and strict: it is type-checked, not run.
import init, {
  blackLitterman,
  Books,
  brinsonFachler,
  factorRisk,
  impliedCovariance,
  karnoskySinger,
  MarketData,
  portfolioFromCsv,
  resampled,
  solve,
  statisticalModel,
  statistics,
  tailRisk,
  type Attribution,
  type BlackLittermanEstimate,
  type CurrencyAttribution,
  type FactorModelEstimate,
  type Matrix,
  type Objective,
  type Problem,
  type ReturnsTable,
  type Solution,
  type Statistics,
} from "../pkg/finalytics.js";
import { createEngine } from "../pkg/client.js";

export async function example(worker: Worker): Promise<void> {
  await init();
  const stats: Statistics = statistics(new Float64Array([0.01, -0.02, 0.005]));
  const sharpe: number | null | undefined = stats.sharpe_ratio;
  const objective: Objective = { objective: "maximum_sharpe", risk_free_rate: 0.02 };
  const solution: Solution = solve(
    {
      labels: ["a", "b"],
      covariance: { labels: ["a", "b"], values: [0.04, 0, 0, 0.09] },
      expected_returns: [0.05, 0.07],
    },
    objective,
  );
  const doubled = solution.weights?.map((w) => w * 2);
  const tail = tailRisk([0.01, -0.02], 0.99, "cornish_fisher");
  const portfolio = portfolioFromCsv("id,kind,currency\n", "id,account,trade_date,type\n", "USD");
  const books = new Books(portfolio, new MarketData());
  const performance = books.performance();
  const returns: Float64Array = performance.returns;
  const threeYears = performance.period("3Y");
  const plan = books.rebalance("main", [[{ holding: "instrument", id: "SPY" }, 1]], { lot_method: "highest_cost" });

  const engine = createEngine(worker);
  const remote = await engine.call("statistics", [0.01, 0.02]);
  const remoteBooks = await engine.books(portfolio);
  const remotePerformance = await remoteBooks.call("performance");
  const remoteReturns: Float64Array = await remotePerformance.get("returns");

  // @ts-expect-error: an unknown objective is rejected.
  const unknown: Objective = { objective: "maximum_profit" };

  console.log(sharpe, doubled, tail, returns, threeYears, plan, remote, remoteReturns, unknown);
}

// Factor models: estimation, risk and a problem with exposure limits.
declare const table: ReturnsTable;
const estimate: FactorModelEstimate = statisticalModel(table, 3);
const exposure: number = factorRisk([0.5, 0.5], estimate.model).factors[0].exposure;
const implied: Matrix = impliedCovariance(estimate.model);
const factorProblem: Problem = {
  labels: estimate.model.assets,
  factor_model: estimate.model,
  constraints: { factor_exposures: [{ factor: "pc1", min: 0.9, max: 1.1 }] },
};
void exposure;
void implied;
void factorProblem;

// Black–Litterman, robust and resampled optimisation.
declare const covariance: Matrix;
const posterior: BlackLittermanEstimate = blackLitterman({
  covariance,
  prior: { prior: "market", weights: [0.6, 0.4], risk_aversion: 2.5 },
  views: [{ portfolio: { bonds: 1 }, expected_return: 0.05, uncertainty: { method: "confidence", confidence: 0.7 } }],
});
const robust: Objective = {
  objective: "robust_mean_variance",
  risk_aversion: 3,
  uncertainty: { set: "ellipsoid", size: 0.5 },
};
const average: number[] | null | undefined = resampled(
  { labels: posterior.labels, covariance: posterior.posterior_covariance, expected_returns: posterior.posterior_returns },
  robust,
  { samples: 100, observations: 120, seed: 7 },
).weights;
void average;

// Attribution.
const attribution: Attribution = brinsonFachler(
  [
    {
      start: "2026-01-01",
      end: "2026-12-31",
      segments: [{ name: "UK", portfolio_weight: 1, portfolio_return: 0.2, benchmark_weight: 1, benchmark_return: 0.1 }],
    },
  ],
  "menchero",
);
const byCurrency: CurrencyAttribution = karnoskySinger([
  {
    start: "2026-01-01",
    end: "2026-12-31",
    segments: [
      { name: "Europe", currency: "EUR", portfolio_weight: 1, portfolio_return: 0.05, benchmark_weight: 1, benchmark_return: 0.04 },
    ],
    currencies: [
      { currency: "USD", spot_return: 0, cash_return: 0.04 },
      { currency: "EUR", spot_return: 0.03, cash_return: 0.025 },
    ],
    portfolio_exposures: [["USD", 0.5], ["EUR", 0.5]],
  },
]);
// @ts-expect-error: an unknown linking method is rejected
brinsonFachler([], "geometric");
void attribution.segments[0].allocation;
void byCurrency.residual;
