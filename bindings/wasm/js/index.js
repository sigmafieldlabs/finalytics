// The entry point for browsers and bundlers: call `await init()` once before using the engine.
// The S&P 500 datasets are fetched from the package's files when first used.
import { install } from "./datasets.js";

install(async (url) => {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`could not load ${url}: ${response.status} ${response.statusText}`);
  }
  return response.text();
});

export * from "./finalytics.js";
export { default } from "./finalytics.js";
