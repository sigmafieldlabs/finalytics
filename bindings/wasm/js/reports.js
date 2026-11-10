// The reports and results the engine returns are plain data, with methods added: a report has
// `toText`, `toHtml`, `saveHtml`, `show` and `toString`; the results of `Assets.optimise`, `Assets.evaluate`, `Assets.compare`,
// `Assets.backtest`, `Assets.plan`, `Portfolio.rebalance` and `Universe.screen` have the same methods, for their
// report, with the report in `report` and the raw results, which print as text too, in `results`. The methods
// are not enumerable, so the data still writes to JSON and passes to a worker as data alone.
import { Assets, Portfolio, Universe, reportHtml, reportText, resultsText } from "./finalytics.js";

// What `show()` does with a report's page when it is given no element, if the program says.
let showHandler = null;

/**
 * Sets what `show()` does with a report's page when it is given no element: from then on it calls
 * `handler` with the page's HTML, instead of opening a window or a browser, and returns
 * `undefined`. `null` (or nothing) restores the default. For programs that show pages themselves,
 * such as an engine in a worker, which has no page of its own, or an app with a view for reports.
 */
export function setShowHandler(handler) {
  if (handler !== null && handler !== undefined && typeof handler !== "function") {
    // Refused as the engine refuses its inputs: a ValidationError with the problem at its path.
    const message = "must be a function that takes the page's HTML, or null";
    const error = new Error(`validation found 1 error(s)\n  error: handler: ${message}`);
    error.name = "ValidationError";
    error.issues = [{ path: "handler", message, severity: "error" }];
    throw error;
  }
  showHandler = handler ?? null;
}

/**
 * Adds the methods to the reports and results: `show` lays out an HTML page, and `save` writes
 * one to a file and returns its path, where there are files.
 */
export function install(show, save) {
  const page = { show, save };
  wrap(Portfolio, "report", (report) => withReportMethods(report, page));
  wrap(Portfolio, "rebalance", (result) => withResultMethods(result, page));
  for (const name of ["optimise", "evaluate", "compare", "backtest", "plan"]) {
    wrap(Assets, name, (result) => withResultMethods(result, page));
  }
  wrap(Universe, "screen", (result) => withResultMethods(result, page, screenRows));
}

/** A screen's columns as the rows of `ScreenResults`, one per security, for printing. */
function screenRows(columns) {
  const measures = Object.keys(columns.measures);
  const fields = Object.keys(columns.fields);
  const value = (x) => (x === null || x === undefined || Number.isNaN(x) ? null : x);
  return {
    as_of: columns.as_of,
    max_carry_days: columns.max_carry_days,
    benchmark: columns.benchmark,
    risk_free_rate: columns.risk_free_rate,
    measures,
    fields,
    formulas: columns.formulas,
    rules: columns.rules,
    sort: columns.sort,
    limit: columns.limit,
    matches: columns.matches,
    coverage: columns.coverage,
    warnings: columns.warnings,
    securities: columns.ids.map((id, i) => ({
      id,
      status: columns.status[i],
      passed: columns.passed[i],
      matched: columns.matched[i],
      rank: value(columns.rank[i]),
      first_date: columns.first_date[i],
      last_date: columns.last_date[i],
      values: measures.map((m) => value(columns.measures[m][i])),
      fields: Object.fromEntries(
        fields.flatMap((f) => (value(columns.fields[f][i]) === null ? [] : [[f, columns.fields[f][i]]])),
      ),
    })),
  };
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

function withReportMethods(report, page) {
  return methods(report, {
    toText(name) {
      return reportText(this, name);
    },
    toHtml(name, theme) {
      return reportHtml(this, name, theme);
    },
    saveHtml(path, name, theme) {
      if (!page.save) {
        throw new Error("saveHtml writes a file, in Node.js; here, use toHtml() for the page");
      }
      return page.save(path, reportHtml(this, name, theme));
    },
    show(element, theme) {
      const html = reportHtml(this, undefined, theme);
      if (!element && showHandler) {
        showHandler(html);
        return undefined;
      }
      return page.show(html, element);
    },
    toString() {
      return reportText(this);
    },
  });
}

function withResultMethods(result, page, rows = (results) => results) {
  withReportMethods(result.report, page);
  methods(result.results, {
    toText() {
      return resultsText(rows(this));
    },
    toString() {
      return resultsText(rows(this));
    },
  });
  return methods(result, {
    toText(name) {
      return this.report.toText(name);
    },
    toHtml(name, theme) {
      return this.report.toHtml(name, theme);
    },
    saveHtml(path, name, theme) {
      return this.report.saveHtml(path, name, theme);
    },
    show(element, theme) {
      return this.report.show(element, theme);
    },
    toString() {
      return this.report.toText();
    },
  });
}
