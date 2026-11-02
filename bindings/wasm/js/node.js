// The entry point for Node.js: loads the engine from the package's WebAssembly file, so that it is
// ready on import, with no call to `init`.
import { readFileSync } from "node:fs";

import { initSync } from "./finalytics.js";

initSync({ module: readFileSync(new URL("./finalytics_bg.wasm", import.meta.url)) });

export * from "./finalytics.js";
export { default } from "./finalytics.js";
