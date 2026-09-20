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
