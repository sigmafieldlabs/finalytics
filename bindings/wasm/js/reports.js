// The reports and results the engine returns are plain data, with methods added: a report has
// `toText`, `toHtml`, `show` and `toString`; the results of `Assets.optimise`, `Assets.compare`,
// `Assets.backtest` and `Portfolio.rebalance` have the same methods, for their report, with the
// report in `report` and the raw results, which print as text too, in `results`. The methods are
// not enumerable, so the data still writes to JSON and passes to a worker as data alone.
import { Assets, Portfolio, reportHtml, reportText, resultsText } from "./finalytics.js";

/** Adds the methods to the reports and results; `show` lays out an HTML page. */
export function install(show) {
  wrap(Portfolio, "report", (report) => withReportMethods(report, show));
  wrap(Portfolio, "rebalance", (result) => withResultMethods(result, show));
  for (const name of ["optimise", "compare", "backtest"]) {
    wrap(Assets, name, (result) => withResultMethods(result, show));
  }
}

function wrap(type, name, add) {
  const original = type.prototype[name];
  type.prototype[name] = function (...args) {
    return add(original.apply(this, args));
  };
}

function methods(target, entries) {
  return Object.defineProperties(
    target,
    Object.fromEntries(Object.entries(entries).map(([name, value]) => [name, { value }])),
  );
}

function withReportMethods(report, show) {
  return methods(report, {
    toText(name) {
      return reportText(this, name);
    },
    toHtml(name) {
      return reportHtml(this, name);
    },
    show(element) {
      return show(reportHtml(this), element);
    },
    toString() {
      return reportText(this);
    },
  });
}

function withResultMethods(result, show) {
  withReportMethods(result.report, show);
  methods(result.results, {
    toText() {
      return resultsText(this);
    },
    toString() {
      return resultsText(this);
    },
  });
  return methods(result, {
    toText(name) {
      return this.report.toText(name);
    },
    toHtml(name) {
      return this.report.toHtml(name);
    },
    show(element) {
      return this.report.show(element);
    },
    toString() {
      return this.report.toText();
    },
  });
}
