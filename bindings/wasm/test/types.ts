// Compiled with `tsc -p bindings/wasm/test` to check that the published declarations are usable
// and strict: it is type-checked, not run.
import init, {
  Assets,
  blackLitterman,
  datasets,
  brinsonFachler,
  factorRisk,
  impliedCovariance,
  karnoskySinger,
  MarketData,
  Portfolio,
  reportTables,
  resampled,
  solve,
  statisticalModel,
  stressedCovariance,
  statistics,
  tailRisk,
  type Attribution,
  type Backtest,
  type BacktestReport,
  type OptimisationReport,
  type BlackLittermanEstimate,
  type Household,
  type CurrencyAttribution,
  type FactorModelEstimate,
  type Matrix,
  type Objective,
  type OptimisationResult,
  type PortfolioReport,
  type PerformanceReport,
  type Problem,
  type ScenarioResult,
  type ReturnsTable,
  type Solution,
  type Statistics,
} from "../pkg/finalytics.js";
import { createEngine } from "../pkg/client.js";

export async function example(worker: Worker): Promise<void> {
  await init();
  const stats: Statistics = statistics(new Float64Array([0.01, -0.02, 0.005]));
  const sharpe: number | null | undefined = stats.sharpe_ratio;
  const objective: Objective = { type: "maximum_sharpe", risk_free_rate: 0.02 };
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
  const books = Portfolio.fromCsv({ transactions: "date,type,amount,currency\n", base_currency: "USD" });
  const portfolio = books.data;
  const performance = books.performance();
  const returns: Float64Array = performance.returns;
  const dates: string[] = performance.dates;
  const threeYears = performance.period("3Y");
  const plan = books.rebalance({ SPY: 1 }, { accounts: ["main"], lot_method: "highest_cost" });
  const traded: Portfolio = books.withRebalance(plan);
  const report: PortfolioReport = traded.report({ benchmark: "SPY", group_by: ["asset_class"] });
  const assets = new Assets(datasets.assetClassPrices(), { risk_free_asset: "T-Bills" });
  const result: OptimisationResult = assets.optimise({
    method: "maximum_sharpe",
    bounds: { min: 0, max: 0.4 },
    groups: { Equity: { assets: ["US Equity", "Intl Equity"], min: 0.3, max: 0.6 } },
  });
  const weights: Record<string, number> | null | undefined = result.weights;
  // @ts-expect-error: an unknown method is rejected.
  assets.optimise({ method: "maximum_profit" });
  const compared = assets.compare({
    methods: ["equal_weight", "risk_parity"],
    max_weight: 0.4,
    rebalance: "quarterly",
    validation: { walk_forward: { years: 5 } },
    trading_cost_bps: 10,
  });
  const single: BacktestReport = assets.backtest({
    method: "maximum_sharpe",
    benchmark: { "US Equity": 0.6, Bonds: 0.4 },
    validation: { holdout: { start: "2016-11-01" } },
  });
  const fixed: BacktestReport = assets.backtest({ weights: { Bonds: 1 }, rebalance: { drift: { band: 0.05 } } });
  const comparedData: OptimisationReport = compared;
  const comparedPage: string = compared.toHtml();
  const raw: Backtest = assets.simulate({ strategies: [{ type: "weights", weights: { Bonds: 1 } }] });
  // @ts-expect-error: an unknown schedule is rejected.
  assets.compare({ methods: ["equal_weight"], rebalance: "weekly" });
  const named = traded.report({ benchmark: "SPY", name: "Main account" });
  const page: string = named.toHtml();
  const shown: string | undefined = named.show();

  const engine = createEngine(worker);
  const remote = await engine.call("statistics", [0.01, 0.02]);
  const remotePortfolio = await engine.portfolio(portfolio);
  const remotePerformance = await remotePortfolio.call("performance");
  const remoteReturns: Float64Array = await remotePerformance.get("returns");

  // @ts-expect-error: an unknown objective is rejected.
  const unknown: Objective = { type: "maximum_profit" };

  console.log(sharpe, doubled, tail, returns, threeYears, plan, remote, remoteReturns, unknown, report, weights);
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
  prior: { type: "market", weights: [0.6, 0.4], risk_aversion: 2.5 },
  views: [{ portfolio: { bonds: 1 }, expected_return: 0.05, uncertainty: { type: "confidence", confidence: 0.7 } }],
});
const robust: Objective = {
  type: "robust_mean_variance",
  risk_aversion: 3,
  uncertainty: { type: "ellipsoid", size: 0.5 },
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
    portfolio_exposures: { USD: 0.5, EUR: 0.5 },
  },
]);
// @ts-expect-error: an unknown linking method is rejected
brinsonFachler([], "geometric");
void attribution.segments[0].allocation;
void byCurrency.residual;

// Scenarios.
declare const books: Portfolio;
const shocked: ScenarioResult = books.shockScenario(
  { groups: [{ scheme: "asset_class", value: "Equity", shock: -0.2 }], currencies: { EUR: -0.1 } },
  { date: "2026-10-30", group_by: ["asset_class"], risk: { covariance, stress: { correlation_shift: 0.5 } } },
);
const replayed: ScenarioResult = books.historicalScenario({ start: "2026-09-01", end: "2026-09-30", proxies: { SAP: "SPY" } });
const stressedMatrix: Matrix = stressedCovariance(covariance, { volatility_scale: 1.5 });
void shocked.holdings[0].weight_after;
void replayed.path?.worst_drawdown?.depth;
void stressedMatrix;

// Households.
const household: Household = books.household(undefined, { group_by: ["owner"], timing: "end_of_day" });
const contribution: number = household.accounts[0].contribution;
const twr: number | null | undefined = household.groups[0].groups[0].time_weighted;
void contribution;
void twr;
void books.accountSeries().lines[0].flows;

// Reports.
const report: PerformanceReport = books.performanceReport({ benchmark: "SPX", group_by: ["asset_class"] });
const september: number | null | undefined = report.monthly.values[0][8];
const exported = reportTables(report);
const firstCell: number | boolean | string | null = exported[0].rows[0][0];
void september;
void firstCell;
void books.source().digest;
