// Reports from `Portfolio.report` are plain data, as the engine returns them, with three methods
// added: `toText`, `toHtml` and `show`. The methods are not enumerable, so a report still writes
// to JSON and passes to a worker as data alone.
import { Portfolio, reportHtml, reportText } from "./finalytics.js";

/** Adds the methods to the reports of `Portfolio.report`; `show` lays out an HTML page. */
export function install(show) {
  const report = Portfolio.prototype.report;
  Portfolio.prototype.report = function (options) {
    return withMethods(report.call(this, options), show);
  };
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
