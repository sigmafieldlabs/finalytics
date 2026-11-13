import { Assets, datasets } from "finalytics";

const assets = new Assets(datasets.assetClassPrices());
const portfolio = datasets.portfolio();
const universe = datasets.universe();

console.log("Assets:", assets.labels.length,
  `['${assets.labels[0]}', '${assets.labels[1]}']`);
console.log("Portfolio:", portfolio.baseCurrency,
  portfolio.data.transactions.length);
console.log("Universe:", universe.length, universe.benchmark);
