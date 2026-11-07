// Calls a Finalytics worker (`worker.js`) with promises.

function listen(worker, receive) {
  if (typeof worker.addEventListener === "function") {
    worker.addEventListener("message", (event) => receive(event.data));
  } else {
    worker.on("message", receive);
  }
}

function rebuild(error) {
  const found = new Error(error.message);
  found.name = error.name;
  if (error.issues !== undefined) {
    found.issues = error.issues;
  }
  return found;
}

/**
 * An engine running in `worker`: every exported function becomes an asynchronous call, and
 * portfolios, assets and performance objects stay in the worker behind handles.
 */
export function createEngine(worker) {
  let next = 1;
  const pending = new Map();
  listen(worker, ({ id, value, error }) => {
    const waiting = pending.get(id);
    if (!waiting) return;
    pending.delete(id);
    if (error) {
      waiting.reject(rebuild(error));
    } else {
      waiting.resolve(value);
    }
  });

  function send(message) {
    const id = next++;
    return new Promise((resolve, reject) => {
      pending.set(id, { resolve, reject });
      worker.postMessage({ id, ...message });
    });
  }

  function remote(found) {
    if (found === null || typeof found !== "object" || !("handle" in found)) {
      return found;
    }
    const { handle } = found;
    return {
      handle,
      call: async (name, ...args) => remote(await send({ op: "method", handle, name, args })),
      get: (name) => send({ op: "get", handle, name }),
      free: () => send({ op: "free", handle }),
    };
  }

  return {
    call: (name, ...args) => send({ op: "call", name, args }),
    portfolio: async (data, market) => remote(await send({ op: "portfolio", args: [data, market] })),
    portfolioFromCsv: async (files) => remote(await send({ op: "portfolioFromCsv", args: [files] })),
    assets: async (prices, options) => remote(await send({ op: "assets", args: [prices, options] })),
    static: async (owner, name, ...args) => remote(await send({ op: "static", owner, name, args })),
    terminate: () => worker.terminate(),
  };
}
