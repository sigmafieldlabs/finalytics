// Runs the JavaScript programs in the package's README as a user would, importing "finalytics"
// from node_modules in a folder that holds the README's CSV files, and checks that each prints
// the output the README shows after it.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { access, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const readme = await readFile(new URL("../README.md", import.meta.url), "utf8");
const pkg = fileURLToPath(new URL("../pkg", import.meta.url));

// The prices that value the README's transactions.
const PRICES = "date,VOO,AGG\n2026-01-02,500,98\n2026-01-05,505,98.2\n";

const blocks = [...readme.matchAll(/```(\w*)\n([\s\S]*?)```/g)].map(([, language, body]) => ({
  language,
  body,
}));

test("the README's programs print what it shows", async () => {
  const programs = blocks.flatMap((block, k) =>
    block.language === "js"
      ? [{ code: block.body, output: blocks[k + 1]?.language === "text" ? blocks[k + 1].body : undefined }]
      : [],
  );
  assert.ok(programs.filter((p) => p.output !== undefined).length >= 2, "the README shows its output");
  const transactions = blocks.find((block) => block.language === "csv");
  assert.ok(transactions, "the README shows a transactions file");

  const dir = await mkdtemp(join(tmpdir(), "finalytics-readme-"));
  try {
    await mkdir(join(dir, "node_modules"));
    await symlink(pkg, join(dir, "node_modules", "finalytics"), "junction");
    await writeFile(join(dir, "transactions.csv"), transactions.body);
    await writeFile(join(dir, "prices.csv"), PRICES);
    for (const [k, { code, output }] of programs.entries()) {
      const file = join(dir, `program-${k + 1}.mjs`);
      await writeFile(file, code);
      const { stdout } = await promisify(execFile)(process.execPath, [file], { cwd: dir });
      if (output !== undefined) {
        assert.equal(stdout, output, `program ${k + 1} prints what the README shows`);
      }
    }
    await access(join(dir, "report.html"));
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
