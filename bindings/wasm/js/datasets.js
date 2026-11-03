// The S&P 500 datasets are files of the package rather than part of the WebAssembly module, so
// that the engine stays small. `install` adds `datasets.sp500()` and `datasets.sp500Index()`,
// which read them on first use with `read`, a function from a file URL to its text.
import { datasets } from "./finalytics.js";

const PRICES = new URL("./data/sp500/prices.csv", import.meta.url);
const INDEX = new URL("./data/sp500/index.csv", import.meta.url);

export function install(read) {
  const cache = new Map();
  const load = (url) => {
    if (!cache.has(url.href)) {
      cache.set(url.href, read(url).then((text) => datasets.__priceTable(text)));
    }
    return cache.get(url.href).then((table) => structuredClone(table));
  };
  datasets.sp500 = () => load(PRICES);
  datasets.sp500Index = () => load(INDEX);
}
