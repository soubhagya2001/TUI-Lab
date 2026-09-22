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

  /** Current screen grid (text, cursor, dimensions; cells when styled). */
  async screen(styled = false): Promise<JsonDict> {
    const action: JsonDict = {
      action: "screen",
      session_id: this.sessionId,
    };
    if (styled) action["styled"] = true;
    return check(await this.conn.request(action), "screen");
  }

  /** Poll until text is visible; raise with the last screen on timeout. */
  async expectText(
    text: string,
    timeoutMs = 10_000,
    regex = false,
    pollMs?: number
  ): Promise<string> {
    const action: JsonDict = {
      action: "wait_for_text",
      session_id: this.sessionId,
      text,
      regex,
      timeout_ms: timeoutMs,
    };
    if (pollMs !== undefined) action["poll_ms"] = pollMs;
    const reply = check(await this.conn.request(action), "wait_for_text");
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

  /**
   * Evaluate a raw engine condition (K1), e.g.
   * `{ type: "text_visible", text: "Dashboard" }` or
   * `{ type: "exit_code", code: 0 }`. Returns the reply
   * (`passed` + `detail`); throws only on transport errors.
   */
  async assert(condition: JsonDict): Promise<JsonDict> {
    const reply = await this.conn.request({
      action: "assert",
      session_id: this.sessionId,
      condition,
    });
    if (reply["ok"] !== true) {
      throw new TuiLabError(
        `assert failed: ${String(reply["error"] ?? "unknown")}`,
        reply
      );
    }
    return reply;
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

  /**
   * Close the session and reap the child (idempotent, K4). `quit` sends
   * quit input first for a graceful exit; `timeoutMs` bounds the wait
   * before kill. A second call returns immediately.
   */
  private closed = false;

  async close(quit?: string, timeoutMs?: number): Promise<JsonDict> {
    if (this.closed) {
      return { ok: true, detail: "already closed" };
    }
    this.closed = true;
    try {
      const action: JsonDict = {
        action: "close",
        session_id: this.sessionId,
      };
      if (quit !== undefined) action["signal"] = quit;
      if (timeoutMs !== undefined) action["timeout_ms"] = timeoutMs;
      return check(await this.conn.request(action), "close");
    } finally {
      await this.conn.close();
    }
  }
}

export class Runner {
  /**
   * Read and parse a results file, rejecting missing or stale output (K3).
   * `startedMs` is the run's start time; a file older than the run is a
   * leftover that must never pass as fresh output.
   */
  static readResultsFile(resultsPath: string, startedMs: number): JsonDict[] {
    if (!fs.existsSync(resultsPath)) {
      throw new TuiLabError("tuilab run produced no reports/results.json");
    }
    if (fs.statSync(resultsPath).mtimeMs < startedMs) {
      throw new TuiLabError("reports/results.json is older than this run (stale results?)");
    }
    return JSON.parse(fs.readFileSync(resultsPath, "utf8")) as JsonDict[];
  }

  /** YAML suites through `tuilab run` (canonical format, docs/05). */
  static async run(
    testFile: string,
    binary?: string
  ): Promise<JsonDict[]> {
    const started = Date.now();
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
    return Runner.readResultsFile(path.resolve("reports/results.json"), started);
  }
}
