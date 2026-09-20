/**
 * JSON-lines sidecar transport: one action per stdin line, one reply out.
 *
 * Speaks the `tuilab proto` contract pinned by
 * `tuilab/crates/cli/tests/proto_roundtrip.rs` — an engine change that
 * breaks SDKs fails there first.
 */
import { ChildProcess, spawn } from "node:child_process";
import * as readline from "node:readline";
import { TuiLabError } from "./errors.js";
import { findBinary } from "./binary.js";

export type JsonDict = Record<string, unknown>;

export class Connection {
  private constructor(
    private readonly proc: ChildProcess,
    private readonly pending: Array<{
      resolve: (reply: JsonDict) => void;
      reject: (err: Error) => void;
      timer: NodeJS.Timeout;
    }> = []
  ) {
    const rl = readline.createInterface({ input: proc.stdout! });
    rl.on("line", (line: string) => {
      const next = this.pending.shift();
      if (!next) return;
      clearTimeout(next.timer);
      try {
        next.resolve(JSON.parse(line) as JsonDict);
      } catch (err) {
        next.reject(
          new TuiLabError(`proto returned non-JSON: ${line}`, {})
        );
      }
    });
    proc.on("error", (err) => this.failAll(err));
    proc.on("exit", () => this.failAll(new Error("tuilab proto exited")));
  }

  static async spawn(binary?: string): Promise<Connection> {
    const proc = spawn(findBinary(binary), ["proto"], {
      stdio: ["pipe", "pipe", "inherit"],
    });
    // tuilab proto emits nothing until the first request, so there is no
    // banner to wait for — fail fast on spawn errors only.
    await new Promise<void>((resolve, reject) => {
      proc.once("error", reject);
      proc.once("spawn", () => {
        proc.removeListener("error", reject);
        resolve();
      });
    });
    return new Connection(proc);
  }

  private failAll(err: Error): void {
    for (const next of this.pending.splice(0)) {
      clearTimeout(next.timer);
      next.reject(err);
    }
  }

  request(action: JsonDict): Promise<JsonDict> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new TuiLabError("proto reply timed out", { action })),
        60_000
      );
      this.pending.push({ resolve, reject, timer });
      this.proc.stdin!.write(JSON.stringify(action) + "\n", (err) => {
        if (err) {
          clearTimeout(timer);
          reject(err);
        }
      });
    });
  }

  async close(): Promise<void> {
    this.proc.stdin!.end();
    await new Promise<void>((resolve) => {
      const done = () => resolve();
      this.proc.once("exit", done);
      setTimeout(() => {
        this.proc.kill();
        resolve();
      }, 10_000).unref?.();
    });
  }
}
