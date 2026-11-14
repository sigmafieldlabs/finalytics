import { datasets, reportText } from "finalytics";

const portfolio = datasets.portfolio();
const options = datasets.reportOptions();
for (const report of [
  portfolio.overviewReport(options),
  portfolio.holdingsReport(options),
  portfolio.performanceReport(options),
  portfolio.riskReport(options),
  portfolio.attributionReport(options),
  portfolio.scenariosReport(options),
]) {
  console.log(reportText(report));
}
