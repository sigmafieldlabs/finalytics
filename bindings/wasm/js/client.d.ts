import type * as finalytics from "./finalytics.js";

/** A message port that a worker speaks through: a browser `Worker` or a Node `Worker`. */
export interface WorkerLike {
  postMessage(message: unknown): void;
  terminate(): unknown;
  addEventListener?(type: "message", listener: (event: { data: unknown }) => void): void;
  on?(type: "message", listener: (data: unknown) => void): void;
}

type Functions = {
  [K in keyof typeof finalytics as (typeof finalytics)[K] extends (...args: never[]) => unknown ? K : never]: (typeof finalytics)[K];
};
type Methods<T> = { [K in keyof T as T[K] extends (...args: never[]) => unknown ? K : never]: T[K] };
type Remote<R> = R extends finalytics.Performance
  ? RemotePerformance
  : R extends finalytics.Portfolio
    ? RemotePortfolio
    : R extends finalytics.Assets
      ? RemoteAssets
      : R;

/** An object held in the worker. */
export interface RemoteObject<T> {
  readonly handle: number;
  /** Calls a method of the object in the worker. */
  call<K extends keyof Methods<T>>(
    name: K,
    ...args: Parameters<Extract<T[K], (...args: never[]) => unknown>>
  ): Promise<Remote<ReturnType<Extract<T[K], (...args: never[]) => unknown>>>>;
  /** Reads a property of the object in the worker. */
  get<K extends keyof T>(name: K): Promise<T[K]>;
  /** Frees the object in the worker. */
  free(): Promise<void>;
}

export type RemotePortfolio = RemoteObject<finalytics.Portfolio>;
export type RemoteAssets = RemoteObject<finalytics.Assets>;
export type RemotePerformance = RemoteObject<finalytics.Performance>;

/** The engine running in a worker. */
export interface Engine {
  /** Calls an exported function in the worker. */
  call<K extends keyof Functions>(name: K, ...args: Parameters<Functions[K]>): Promise<ReturnType<Functions[K]>>;
  /** A portfolio in the worker, from its records and market data in the document format. */
  portfolio(data: finalytics.PortfolioData, market?: finalytics.DocumentMarketData): Promise<RemotePortfolio>;
  /** A portfolio in the worker, from the CSV text of its files. */
  portfolioFromCsv(files: finalytics.PortfolioFiles): Promise<RemotePortfolio>;
  /** Assets in the worker, from their prices. */
  assets(prices: finalytics.PriceTable, options?: finalytics.AssetsOptions): Promise<RemoteAssets>;
  /** Stops the worker. */
  terminate(): unknown;
}

/** An engine running in `worker`, which runs `finalytics/worker.js`. */
export function createEngine(worker: WorkerLike): Engine;
