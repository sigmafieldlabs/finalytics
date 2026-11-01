// Runs the Finalytics engine in a worker, so that long computations do not block the page.
//
// Start it as a module worker and talk to it through `createEngine` in `client.js`:
//
//   const worker = new Worker(new URL("finalytics/worker.js", import.meta.url), { type: "module" });
//   const engine = createEngine(worker);
//
// The worker also runs under Node's `worker_threads`.

import init, * as finalytics from "./finalytics.js";

const inBrowser = typeof self !== "undefined" && typeof self.postMessage === "function";
const port = inBrowser ? self : (await import("node:worker_threads")).parentPort;

async function start() {
  if (inBrowser) {
    return init();
  }
  const { readFile } = await import("node:fs/promises");
  return init({ module_or_path: await readFile(new URL("./finalytics_bg.wasm", import.meta.url)) });
}

const ready = start();
const objects = new Map();
let next = 1;

function keep(value) {
  if (value instanceof finalytics.Books || value instanceof finalytics.Performance) {
    objects.set(next, value);
    return { handle: next++ };
  }
  return value;
}

function target(handle) {
  const found = objects.get(handle);
  if (!found) {
    throw new Error(`the worker holds no object ${handle}; it may have been freed`);
  }
  return found;
}

async function handle(message) {
  const { id, op, name, args = [], handle: which } = message;
  try {
    await ready;
    let value;
    switch (op) {
      case "call":
        value = finalytics[name](...args);
        break;
      case "books": {
        const [portfolio, market] = args;
        const data = market === undefined ? undefined : finalytics.MarketData.fromDocument(market);
        value = keep(new finalytics.Books(portfolio, data));
        break;
      }
      case "method":
        value = keep(target(which)[name](...args));
        break;
      case "get":
        value = target(which)[name];
        break;
      case "free":
        target(which).free();
        objects.delete(which);
        break;
      default:
        throw new Error(`unknown operation ${op}`);
    }
    port.postMessage({ id, value });
  } catch (error) {
    port.postMessage({
      id,
      error: { name: error?.name ?? "Error", message: error?.message ?? String(error), issues: error?.issues },
    });
  }
}

if (inBrowser) {
  self.addEventListener("message", (event) => handle(event.data));
} else {
  port.on("message", handle);
}
