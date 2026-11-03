// The reports of `Portfolio.report`, `Assets.compare` and `Assets.backtest` are plain data, as
// the engine returns them, with three methods added: `toText`, `toHtml` and `show`. The methods
// are not enumerable, so a report still writes to JSON and passes to a worker as data alone.
import { Assets, Portfolio, reportHtml, reportText } from "./finalytics.js";

/** Adds the methods to the reports; `show` lays out an HTML page. */
export function install(show) {
  for (const [type, name] of [
    [Portfolio, "report"],
    [Assets, "compare"],
    [Assets, "backtest"],
  ]) {
    const original = type.prototype[name];
    type.prototype[name] = function (options) {
      return withMethods(original.call(this, options), show);
    };
  }
}

function withMethods(report, show) {
  return Object.defineProperties(report, {
    toText: {
      value(name) {
        return reportText(this, name);
      },
    },
    toHtml: {
      value(name) {
        return reportHtml(this, name);
      },
    },
    show: {
      value(element) {
        return show(reportHtml(this), element);
      },
    },
  });
}
