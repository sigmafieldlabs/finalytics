// Runs the quick start in the package's README as a user would, importing "finalytics" from
// node_modules, and checks that it prints what its comments show.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const readme = await readFile(new URL("../README.md", import.meta.url), "utf8");
const pkg = fileURLToPath(new URL("../pkg", import.meta.url));

test("the README's quick start prints what it shows", async () => {
  const block = readme.match(/## Quick start[\s\S]*?```js\n([\s\S]*?)```/)?.[1];
  assert.ok(block, "the README has a JavaScript quick start");
  const expected = block
    .split("\n")
    .filter((line) => line.startsWith("console.log("))
    .map((line) => line.slice(line.lastIndexOf("// ") + 3));
  assert.ok(expected.length > 0, "the quick start shows its output");

  const dir = await mkdtemp(join(tmpdir(), "finalytics-readme-"));
  try {
    await mkdir(join(dir, "node_modules"));
    await symlink(pkg, join(dir, "node_modules", "finalytics"), "junction");
    await writeFile(join(dir, "quick-start.mjs"), block);
    const { stdout } = await promisify(execFile)(process.execPath, [join(dir, "quick-start.mjs")]);
    assert.deepEqual(stdout.trimEnd().split("\n"), expected);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
});
