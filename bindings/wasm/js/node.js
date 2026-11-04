// The entry point for Node.js: loads the engine from the package's WebAssembly file, so that it is
// ready on import, with no call to `init`. The S&P 500 datasets are read from the package's
// files when first used, and reports show in the default browser.
import { spawn } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";

import { install as installDatasets } from "./datasets.js";
import { initSync } from "./finalytics.js";
import { install as installReports } from "./reports.js";

initSync({ module: readFileSync(new URL("./finalytics_bg.wasm", import.meta.url)) });
installDatasets((url) => readFile(url, "utf8"));

installReports(
  (html) => {
    const path = join(tmpdir(), `finalytics-report-${Date.now()}.html`);
    writeFileSync(path, html);
    const [command, ...args] = opener(path, process.env.BROWSER);
    const child = spawn(command, args, { detached: true, stdio: "ignore" });
    child.on("error", (error) => {
      process.emitWarning(`cannot open a browser (${error.message}); the report is at ${path}`);
    });
    child.unref();
    return path;
  },
  (path, html) => {
    writeFileSync(path, html);
    return path;
  },
);

/**
 * The command that opens `path`: the first of the commands in `browser` (the `BROWSER`
 * environment variable), with `%s` standing for the path if it appears, or else the system's
 * default for HTML files.
 */
function opener(path, browser) {
  const first = (browser ?? "").split(delimiter)[0].trim();
  if (first) {
    const words = first.split(/\s+/);
    const placed = words.some((word) => word.includes("%s"));
    const args = words.slice(1).map((word) => word.replaceAll("%s", path));
    return [words[0], ...args, ...(placed ? [] : [path])];
  }
  if (process.platform === "darwin") return ["open", path];
  if (process.platform === "win32") return ["cmd", "/c", "start", "", path];
  return ["xdg-open", path];
}

export * from "./finalytics.js";
export { default } from "./finalytics.js";
