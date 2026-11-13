import { readFile, writeFile, unlink } from "node:fs/promises";
import { Portfolio, datasets } from "finalytics";

const path = "saved-portfolio.json";
await writeFile(path, datasets.portfolio().toJson(), "utf8");
const restored = Portfolio.fromJson(await readFile(path, "utf8"));
console.log("Restored:", restored.baseCurrency,
  restored.data.transactions.length);
await unlink(path);
