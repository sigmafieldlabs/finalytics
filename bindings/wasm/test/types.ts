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
  Universe,
  type Attribution,
  type BacktestReport,
  type BacktestResults,
  type ComparisonReport,
  type OptimisationReport,
  type BlackLittermanEstimate,
  type Household,
  type CurrencyAttribution,
  type FactorModelEstimate,
  type Matrix,
  type Objective,
  type OptimisationResult,
  type RebalanceReport,
  type RebalanceResults,
  type PortfolioReport,
  type PerformanceReport,
  type Problem,
  type ScenarioResult,
  type ReturnsTable,
  type ScreenColumns,
  type ScreenReport,
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
  const plan = books.rebalance({ SPY: 1 }, { accounts: ["main"], lot_method: "highest_cost", name: "Main", as_of: "2026-06-30" });
  const planReport: RebalanceReport = plan.report;
  const planResults: RebalanceResults = plan.results;
  const planText: string = `${plan}` + plan.results.toText() + plan.toHtml();
  const traded: Portfolio = books.withRebalance(plan);
  const tradedAgain: Portfolio = books.withRebalance(plan.results);
  const report: PortfolioReport = traded.report({ benchmark: "SPY", group_by: ["asset_class"] });
  const assets = new Assets(datasets.assetClassPrices(), { risk_free_asset: "T-Bills" });
  const result = assets.optimise({
    method: "maximum_sharpe",
    bounds: { min: 0, max: 0.4 },
    groups: { Equity: { assets: ["US Equity", "Intl Equity"], min: 0.3, max: 0.6 } },
  });
  const optimisation: OptimisationReport = result.report;
  const solved: OptimisationResult = result.results;
  const weights: Record<string, number> | null | undefined = result.results.weights;
  const optimisationText: string = `${result}` + result.results.toText() + result.toText("Named");
  const saved: string = assets.optimise({ method: "minimum_variance", name: "Defensive" }).saveHtml("page.html");
  const frontierWeights: Record<string, number> = assets.frontier(5)[0].weights;
  const rebalanced = books.rebalance(result);
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
  }).report;
  const fixed = assets.backtest({ weights: { Bonds: 1 }, rebalance: { drift: { band: 0.05 } } });
  const fixedRuns: BacktestResults = fixed.results;
  const comparedData: ComparisonReport = compared.report;
  const comparedPage: string = compared.toHtml();
  const raw: BacktestResults = compared.results;
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
  const sampleFiles: string[] = await engine.static("datasets", "files");
  const remoteAssets = await engine.static("Assets", "fromCsv", "date,X\n2026-01-02,10\n");
  const remoteLabels: string[] = await remoteAssets.get("labels");
  // @ts-expect-error: only the listed classes have static methods in the worker.
  void engine.static("Window", "open");
  const remoteUniverse = await engine.universe([{ id: "A", dates: ["2026-10-30"], close: [10] }]);
  const remoteScreen = await remoteUniverse.call("screen", { limit: 5 });
  const remoteColumns = await engine.static("Universe", "fromColumns", {
    ids: ["A"],
    starts: [0],
    dates: new Int32Array([20_000]),
    close: new Float64Array([10]),
  });
  const remoteIds: string[] = await remoteColumns.get("ids");
  void remoteScreen.results.matches;
  void remoteIds;

  const universe = Universe.fromColumns(
    {
      ids: ["A", "B"],
      starts: new Int32Array([0, 2]),
      dates: new Int32Array([20_000, 20_001, 20_001]),
      close: new Float64Array([10, 11, 20]),
      volume: new Float64Array([1e6, 2e6, NaN]),
      fields: { sector: ["Energy", null], pe_ratio: new Float64Array([12, NaN]) },
    },
    { benchmark: "B", risk_free_rate: 0.03 },
  );
  const screen = universe.screen({
    as_of: "2026-10-30",
    measures: ["return_1y", "rsi_14"],
    rules: [
      { measure: "return_1y", op: ">", value: 0.1 },
      { measure: "rsi_14", op: "between", value: [30, 70] },
      { field: "sector", op: "not_in", values: ["Utilities"] },
    ],
    sort: { measure: "sharpe_ratio", order: "descending" },
    limit: 20,
    name: "Momentum",
  });
  const screenReport: ScreenReport = screen.report;
  const columns: ScreenColumns = screen.results;
  const oneYear: Float64Array | undefined = columns.measures.return_1y;
  const screenText: string = `${screen}` + screen.results.toText() + screen.toHtml("Named");
  const fromSeries = new Universe([{ id: "A", dates: ["2026-10-29", "2026-10-30"], close: [10, 11], fields: { sector: "Energy" } }]);
  const withFields: Universe = fromSeries.withFields({ A: { pe_ratio: 12 } }).withFieldsCsv("id,industry\nA,Oil\n");
  // @ts-expect-error: an unknown measure is rejected.
  universe.screen({ measures: ["return_2y"] });
  // @ts-expect-error: an unknown comparison is rejected.
  universe.screen({ rules: [{ measure: "return_1y", op: "=", value: 0 }] });
  console.log(screenReport, oneYear, screenText, withFields);

  // @ts-expect-error: an unknown objective is rejected.
  const unknown: Objective = { type: "maximum_profit" };

  console.log(sharpe, doubled, tail, returns, threeYears, plan, remote, remoteReturns, unknown, report, weights);
  console.log(sampleFiles, remoteLabels);
  console.log(optimisation, solved, optimisationText, rebalanced, single, fixedRuns, comparedData, comparedPage, raw);
  console.log(planReport, planResults, planText, tradedAgain);
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
