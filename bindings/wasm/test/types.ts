// Compiled with `tsc -p bindings/wasm/test` to check that the published declarations are usable
// and strict: it is type-checked, not run.
import init, {
  Books,
  MarketData,
  portfolioFromCsv,
  solve,
  statistics,
  tailRisk,
  type Objective,
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
