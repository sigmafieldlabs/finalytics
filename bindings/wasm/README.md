# Finalytics for JavaScript and TypeScript

Finalytics is a portfolio analytics engine: it keeps the books of a multi-currency portfolio from
its transactions, measures performance and risk, and optimises portfolios under real-world
constraints. This package runs the engine as WebAssembly in browsers, workers and Node.js, with
TypeScript types included.

> **Status:** 1.0 is in active development (`1.0.0-alpha`).

```sh
npm install finalytics
```

```ts
import init, { version } from "finalytics";

await init();
console.log(version());
```

- Source and issues: https://github.com/Nnamdi-sys/finalytics
- Licence: MIT OR Apache-2.0
