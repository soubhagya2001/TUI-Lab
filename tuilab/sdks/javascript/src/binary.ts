/** Locate the `tuilab` binary: explicit → TUILAB_BIN → @tui-lab/cli → workspace → PATH. */
import { existsSync } from "node:fs";
import * as path from "node:path";
import { TuiLabError } from "./errors.js";

/**
 * Platform tag for the bundled-engine package, or `null` when the
 * platform/arch has no published package.
 *
 * K5: an unknown arch must never collapse to `x64` — that silently loads
 * a foreign binary. Unsupported hosts fall through to the workspace/PATH
 * lookups instead (a self-built engine can still work there).
 */
function platformKey(): string | null {
  const arch = process.arch;
  if (arch !== "x64" && arch !== "arm64") return null;
  return `${process.platform}-${arch}`;
}

/** Engine bundled via the @tui-lab/cli platform package, if installed. */
function cliPackageBinary(): string | null {
  const key = platformKey();
  if (!key) return null;
  const file = process.platform === "win32" ? "tuilab.exe" : "tuilab";
  try {
    const pkg = require.resolve(`@tui-lab/cli-${key}/package.json`, {
      paths: [__dirname],
    });
    const candidate = path.join(path.dirname(pkg), file);
    return existsSync(candidate) ? candidate : null;
  } catch {
    return null;
  }
}

export function findBinary(explicit?: string): string {
  if (explicit) {
    if (existsSync(explicit)) return explicit;
    throw new TuiLabError(`tuilab binary not found: ${explicit}`);
  }
  const env = process.env["TUILAB_BIN"];
  if (env && existsSync(env)) return env;
  // Installed platform package wins over PATH: a PATH `tuilab` may be the
  // @tui-lab/cli node launcher, which must never be spawned as the engine.
  // (The launcher itself resolves the platform package directly, so no loop.)
  const packaged = cliPackageBinary();
  if (packaged) return packaged;
  // dist/binary.js → sdks/javascript → sdks → tuilab → target/debug
  const workspace = path.resolve(
    __dirname,
    "..",
    "..",
    "..",
    "target",
    "debug",
    process.platform === "win32" ? "tuilab.exe" : "tuilab"
  );
  if (existsSync(workspace)) return workspace;
  for (const dir of (process.env["PATH"] ?? "").split(path.delimiter)) {
    const candidate = path.join(
      dir,
      process.platform === "win32" ? "tuilab.exe" : "tuilab"
    );
    if (existsSync(candidate)) return candidate;
  }
  throw new TuiLabError(
    "tuilab binary not found: set TUILAB_BIN, add it to PATH, " +
      "or build the workspace (cargo build -p tui-lab-cli)"
  );
}
