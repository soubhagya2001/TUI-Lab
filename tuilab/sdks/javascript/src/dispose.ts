/**
 * Explicit-resource-management symbols with Node's own fallbacks.
 *
 * `Symbol.dispose` / `Symbol.asyncDispose` landed in Node 20.5; earlier
 * 20.x runtimes get the `nodejs.*` registry keys Node itself uses, so
 * `await using` / `using` keep working instead of silently degrading.
 */
export const disposeSymbol: symbol =
  (Symbol as { dispose?: symbol }).dispose ?? Symbol.for("nodejs.dispose");

export const asyncDisposeSymbol: symbol =
  (Symbol as { asyncDispose?: symbol }).asyncDispose ?? Symbol.for("nodejs.asyncDispose");
