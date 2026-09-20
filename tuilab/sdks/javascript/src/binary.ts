/** Locate the `tuilab` binary: explicit → TUILAB_BIN → PATH → workspace. */
import { existsSync } from "node:fs";
import * as path from "node:path";
import { TuiLabError } from "./errors.js";

export function findBinary(explicit?: string): string {
  if (explicit) {
    if (existsSync(explicit)) return explicit;
    throw new TuiLabError(`tuilab binary not found: ${explicit}`);
  }
  const env = process.env["TUILAB_BIN"];
  if (env && existsSync(env)) return env;
  for (const dir of (process.env["PATH"] ?? "").split(path.delimiter)) {
    const candidate = path.join(
      dir,
      process.platform === "win32" ? "tuilab.exe" : "tuilab"
    );
    if (existsSync(candidate)) return candidate;
  }
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
  throw new TuiLabError(
    "tuilab binary not found: set TUILAB_BIN, add it to PATH, " +
      "or build the workspace (cargo build -p tui-lab-cli)"
  );
}
