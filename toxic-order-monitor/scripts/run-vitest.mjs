import { realpathSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = realpathSync(process.cwd());
process.chdir(root);

const [major, minor] = process.versions.node.split(".").map(Number);
if (major < 20 || (major === 20 && minor < 19)) {
  console.error(
    `Frontend tests require Node.js 20.19+ (or 22.12+); found ${process.versions.node}.`,
  );
  process.exit(1);
}

const command = process.execPath;
const vitestEntry = join(root, "node_modules", "vitest", "vitest.mjs");
const result = spawnSync(command, [vitestEntry, "run", ...process.argv.slice(2)], {
  cwd: root,
  stdio: "inherit",
  shell: false,
});

if (result.error) {
  console.error(result.error);
}

process.exit(result.status ?? 1);
