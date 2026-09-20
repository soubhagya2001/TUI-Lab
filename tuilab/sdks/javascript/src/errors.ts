/** Engine or transport failure, with step/session context attached. */
export class TuiLabError extends Error {
  readonly detail: Record<string, unknown>;

  constructor(message: string, detail: Record<string, unknown> = {}) {
    super(message);
    this.name = "TuiLabError";
    this.detail = detail;
  }
}
