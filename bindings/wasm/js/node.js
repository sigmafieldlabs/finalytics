// The entry point for Node.js: loads the engine from the package's WebAssembly file, so that it is
// ready on import, with no call to `init`. The S&P 500 datasets are read from the package's
// files when first used.
import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";

import { install } from "./datasets.js";
import { initSync } from "./finalytics.js";

initSync({ module: readFileSync(new URL("./finalytics_bg.wasm", import.meta.url)) });
install((url) => readFile(url, "utf8"));

export * from "./finalytics.js";
export { default } from "./finalytics.js";
