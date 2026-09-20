/** Async TUI session over the sidecar (docs/09 §9.3 usage). */
import { execFile } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import { promisify } from "node:util";
import { Connection, JsonDict } from "./proto.js";
import { TuiLabError } from "./errors.js";
import { findBinary } from "./binary.js";

const execFileAsync = promisify(execFile);

function check(reply: JsonDict, action: string): JsonDict {
  if (reply["ok"] !== true) {
    throw new TuiLabError(
      `${action} failed: ${String(reply["error"] ?? "unknown")}`,
      reply
    );
  }
  return reply;
}

export interface LaunchOptions {
  args?: string[];
  cwd?: string;
  env?: Record<string, string>;
  width?: number;
  height?: number;
  binary?: string;
}

export class TuiTest {
  private constructor(
    private readonly conn: Connection,
    readonly sessionId: string
  ) {}

  /** Launch an app under a fresh PTY; returns a live session. */
  static async launch(
    command: string,
    options: LaunchOptions = {}
  ): Promise<TuiTest> {
    const conn = await Connection.spawn(options.binary);
    try {
      const reply = check(
        await conn.request({
          action: "launch",
          command,
          args: options.args ?? [],
          cwd: options.cwd ?? null,
          terminal: {
            width: options.width ?? 120,
            height: options.height ?? 40,
          },
          env: options.env ?? {},
        }),
        "launch"
      );
      return new TuiTest(conn, String(reply["session_id"]));
    } catch (err) {
      await conn.close();
      throw err;
    }
  }

  /** Release the session even when the block raises. */
  async dispose(): Promise<void> {
    await this.close();
  }

  /** Send a named key; returns whether the screen changed. */
  async press(key: string): Promise<boolean> {
    const reply = check(
      await this.conn.request({
        action: "press",
        session_id: this.sessionId,
        key,
      }),
      "press"
    );
    return Boolean(reply["screen_changed"] ?? false);
  }

  /** Type text verbatim (sensitive redacts it from logs). */
  async type(text: string, sensitive = false): Promise<void> {
    check(
      await this.conn.request({
        action: "type",
        session_id: this.sessionId,
        text,
        sensitive,
      }),
      "type"
    );
  }

  /** Current screen grid (text, cursor, dimensions). */
  async screen(): Promise<JsonDict> {
    return check(
      await this.conn.request({
        action: "screen",
        session_id: this.sessionId,
      }),
      "screen"
    );
  }

  /** Poll until text is visible; raise with the last screen on timeout. */
  async expectText(
    text: string,
    timeoutMs = 10_000,
    regex = false
  ): Promise<string> {
    const reply = check(
      await this.conn.request({
        action: "wait_for_text",
        session_id: this.sessionId,
        text,
        regex,
        timeout_ms: timeoutMs,
      }),
      "wait_for_text"
    );
    if (reply["found"] !== true) {
      throw new TuiLabError(`timed out waiting for ${JSON.stringify(text)}`, reply);
    }
    return String(reply["screen"] ?? "");
  }

  /** Assert text is absent from the current screen. */
  async expectNotText(text: string): Promise<void> {
    const screen = await this.screen();
    if (String(screen["text"] ?? "").includes(text)) {
      throw new TuiLabError(`unexpected visible text ${JSON.stringify(text)}`, screen);
    }
  }

  /** Capture a named snapshot (golden written on first use). */
  async snapshot(name: string): Promise<JsonDict> {
    return check(
      await this.conn.request({
        action: "snapshot",
        session_id: this.sessionId,
        name,
      }),
      "snapshot"
    );
  }

  /** Resize the terminal. */
  async resize(width: number, height: number): Promise<void> {
    check(
      await this.conn.request({
        action: "resize",
        session_id: this.sessionId,
        width,
        height,
      }),
      "resize"
    );
  }

  /** Close the session and reap the child. */
  async close(): Promise<JsonDict> {
    try {
      return check(
        await this.conn.request({
          action: "close",
          session_id: this.sessionId,
        }),
        "close"
      );
    } finally {
      await this.conn.close();
    }
  }
}

export class Runner {
  /** YAML suites through `tuilab run` (canonical format, docs/05). */
  static async run(
    testFile: string,
    binary?: string
  ): Promise<JsonDict[]> {
    const { stdout } = await execFileAsync(findBinary(binary), [
      "run",
      testFile,
    ]).catch((err: { code?: number; stdout?: string }) => {
      const code = err.code ?? 1;
      if (code !== 0 && code !== 1) {
        throw new TuiLabError(`tuilab run exited ${code}: ${(err.stdout ?? "").slice(-2000)}`);
      }
      return { stdout: err.stdout ?? "" } as { stdout: string };
    });
    void stdout;
    // Results land next to the invoker's CWD (reports/results.json).
    const resultsPath = path.resolve("reports/results.json");
    if (!fs.existsSync(resultsPath)) {
      throw new TuiLabError("tuilab run produced no reports/results.json");
    }
    return JSON.parse(fs.readFileSync(resultsPath, "utf8")) as JsonDict[];
  }
}
