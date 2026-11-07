// Runs the JavaScript programs in the package's README as a user would, importing "finalytics"
// from node_modules in a folder that holds the portfolio dataset's files and, in tests/sp500, the
// S&P 500 prices. A block that starts with an import starts a new program; the blocks after it
// continue that program. Each program must run, and each that the README follows with a text
// block must print that text.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const readme = await readFile(new URL("../README.md", import.meta.url), "utf8");
const pkg = fileURLToPath(new URL("../pkg", import.meta.url));
const dataset = fileURLToPath(new URL("../../../crates/finalytics/data/portfolio", import.meta.url));
const sp500 = fileURLToPath(new URL("../../../tests/sp500", import.meta.url));

const blocks = [...readme.matchAll(/```(\w*)\n([\s\S]*?)```/g)].map(([, language, body]) => ({
  language,
  body,
}));

/** The README's programs: each `js` block that starts with an import, with the blocks after it. */
function programs() {
  const found = [];
  for (const [k, block] of blocks.entries()) {
    if (block.language !== "js") continue;
    const output = blocks[k + 1]?.language === "text" ? blocks[k + 1].body : undefined;
    if (block.body.startsWith("import ") || found.length === 0) {
      found.push({ code: block.body, output });
    } else {
      const last = found[found.length - 1];
      last.code += `\n${block.body}`;
      last.output = output ?? last.output;
    }
  }
  return found;
}

test("the README's programs run and print what it shows", async () => {
  const found = programs();
  assert.ok(found.length >= 2, "the README has a program for each part");

  const dir = await mkdtemp(join(tmpdir(), "finalytics-readme-"));
  try {
    await mkdir(join(dir, "node_modules"));
    await symlink(pkg, join(dir, "node_modules", "finalytics"), "junction");
    for (const file of await readdir(dataset)) {
      if (file.endsWith(".csv")) await copyFile(join(dataset, file), join(dir, file));
    }
    await mkdir(join(dir, "tests", "sp500"), { recursive: true });
    for (const file of await readdir(sp500)) {
      if (file.endsWith(".csv")) await copyFile(join(sp500, file), join(dir, "tests", "sp500", file));
    }
    for (const [k, { code, output }] of found.entries()) {
      const file = join(dir, `program-${k + 1}.mjs`);
      await writeFile(file, code);
      const { stdout } = await promisify(execFile)(process.execPath, [file], {
        cwd: dir,
        env: { ...process.env, BROWSER: "true" },
      });
      if (output !== undefined) {
        assert.equal(stdout, output, `program ${k + 1} prints what the README shows`);
      }
    }
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
