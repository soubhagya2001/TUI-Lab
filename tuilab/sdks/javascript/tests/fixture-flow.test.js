/** Async SDK flow against the Ratatui fixture (mirrors the Python suite). */
const { describe, it, before } = require("node:test");
const assert = require("node:assert/strict");
const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { TuiTest, Runner, TuiLabError, findBinary } = require("../dist/index.js");

const WORKSPACE = path.resolve(__dirname, "..", "..", "..");
const FIXTURE_DIR = path.join(WORKSPACE, "tests", "fixtures", "ratatui-sample");
const EXE = process.platform === "win32" ? "ratatui-sample.exe" : "ratatui-sample";

let FIXTURE = "";

function fixtureBin() {
  // No --offline: fresh runners must fetch the fixture's deps on demand.
  execFileSync("cargo", ["build"], { cwd: FIXTURE_DIR, stdio: "pipe" });
  // Forward slashes: safe inside YAML double-quoted scalars on Windows.
  return path.join(FIXTURE_DIR, "target", "debug", EXE).replace(/\\/g, "/");
}

before(() => {
  FIXTURE = fixtureBin();
});

describe("fixture flow", () => {
  it("launches, interacts, asserts, and closes", async () => {
    const session = await TuiTest.launch(FIXTURE);
    try {
      await session.expectText("TUI-LAB-SAMPLE");
      await session.press("DOWN");
      await session.press("ENTER");
      const screen = await session.expectText("Selected: beta-chair");
      assert.ok(screen.includes("beta-chair"));
      await session.resize(80, 24);
      await session.press("ESC");
      await session.expectText("TUI-LAB-SAMPLE");
      await session.press("/");
      await session.type("table");
      await session.expectText("Search: table");
      await session.expectNotText("beta-chair");
      const snap = await session.snapshot("js-proof");
      assert.equal(snap.ok, true);
    } finally {
      await session.close();
    }
  });

  it("unknown keys raise TuiLabError", async () => {
    const session = await TuiTest.launch(FIXTURE);
    try {
      await session.expectText("TUI-LAB-SAMPLE");
      await assert.rejects(() => session.press("F13"), TuiLabError);
    } finally {
      await session.close();
    }
  });

  it("wait timeouts carry the last screen", async () => {
    const session = await TuiTest.launch(FIXTURE);
    try {
      await assert.rejects(
        session.expectText("no-such-screen", 500),
        (err) => err instanceof TuiLabError && "screen" in err.detail
      );
    } finally {
      await session.close();
    }
  });

  it("Runner runs YAML suites", async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tuilab-js-"));
    const suite = path.join(dir, "mini.yaml");
    fs.writeFileSync(
      suite,
      `schema: tui-lab/v1
name: mini
application:
  command: "${FIXTURE}"
steps:
  - wait_for_text:
      text: "TUI-LAB-SAMPLE"
  - press: q
assertions:
  - exit_code: 0
`
    );
    const results = await Runner.run(suite);
    assert.ok(Array.isArray(results) && results.length > 0);
    assert.ok(results.every((s) => s.passed));
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("findBinary resolves", () => {
    assert.ok(fs.existsSync(findBinary()));
  });
});

describe("K1-K4 surface", () => {
  it("assert() passes conditions through", async () => {
    const session = await TuiTest.launch(FIXTURE);
    try {
      await session.expectText("TUI-LAB-SAMPLE");
      const passed = await session.assert({ type: "text_visible", text: "TUI-LAB-SAMPLE" });
      assert.equal(passed.passed, true);
      const failed = await session.assert({ type: "text_visible", text: "no-such-screen" });
      assert.equal(failed.passed, false);
      await assert.rejects(session.assert({ type: "no-such-condition" }), TuiLabError);
    } finally {
      await session.close();
    }
  });

  it("screen styled returns cells, waits accept pollMs", async () => {
    const session = await TuiTest.launch(FIXTURE);
    try {
      const screen = await session.expectText("TUI-LAB-SAMPLE", 10_000, false, 25);
      assert.ok(screen.includes("TUI-LAB-SAMPLE"));
      const styled = await session.screen(true);
      assert.ok(Array.isArray(styled.cells) && styled.cells.length > 0);
    } finally {
      await session.close();
    }
  });

  it("close with quit is clean and idempotent", async () => {
    const session = await TuiTest.launch(FIXTURE);
    await session.expectText("TUI-LAB-SAMPLE");
    const first = await session.close("q");
    assert.equal(first.ok, true);
    const second = await session.close();
    assert.equal(second.ok, true);
    assert.equal(second.detail, "already closed");
  });

  it("Runner rejects stale results", () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tuilab-js-stale-"));
    const target = path.join(dir, "results.json");
    fs.writeFileSync(target, "[]");
    const old = Date.now() - 60_000;
    fs.utimesSync(target, new Date(old), new Date(old));
    assert.throws(() => Runner.readResultsFile(target, Date.now()), /stale/);
    fs.writeFileSync(target, '[{"passed": true}]');
    assert.deepEqual(Runner.readResultsFile(target, Date.now() - 1000), [{ passed: true }]);
    assert.throws(() => Runner.readResultsFile(path.join(dir, "missing.json"), 0), /no reports/);
    fs.rmSync(dir, { recursive: true, force: true });
  });
});
