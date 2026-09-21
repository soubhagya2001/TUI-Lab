"use strict";
/* Resolve the engine binary for the launchers.
 *
 * Order: TUILAB_BIN (env override) -> sibling @tui-lab/cli-<platform> package
 * -> error. PATH is deliberately NOT consulted: a PATH `tuilab` may be this
 * very launcher (node_modules/.bin shim), which would re-enter forever.
 */
const { existsSync } = require("node:fs");
const path = require("node:path");
const { spawnSync } = require("node:child_process");

function platformKey() {
  const arch = process.arch === "arm64" ? "arm64" : "x64";
  return `${process.platform}-${arch}`;
}

function binaryFile(kind) {
  return process.platform === "win32" ? `${kind}.exe` : kind;
}

function resolveBinary(kind) {
  const file = binaryFile(kind);
  const envName = kind === "tuilab" ? "TUILAB_BIN" : "TUILAB_MCP_BIN";
  const env = process.env[envName];
  if (env && existsSync(env)) return env;
  try {
    const pkg = require.resolve(`@tui-lab/cli-${platformKey()}/package.json`, {
      paths: [__dirname, process.cwd()],
    });
    const candidate = path.join(path.dirname(pkg), file);
    if (existsSync(candidate)) return candidate;
  } catch {
    // Platform package absent; fall through to the error below.
  }
  throw new Error(
    `${file} not found for ${platformKey()}: install @tui-lab/cli (pulls the ` +
      `platform package), set ${envName}, or build the workspace ` +
      `(cargo build -p tui-lab-cli)`
  );
}

function main(kind) {
  let bin;
  try {
    bin = resolveBinary(kind);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
    return;
  }
  const { status, error } = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
  if (error) {
    console.error(error.message);
    process.exitCode = 1;
    return;
  }
  process.exitCode = status ?? 1;
}

module.exports = { main, resolveBinary, platformKey };
