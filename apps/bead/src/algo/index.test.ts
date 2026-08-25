import { describe, expect, it } from "vitest";

import { ALGO_OWNERS, NOT_IMPLEMENTED, NotImplementedError, notImplemented } from "./index.ts";

// The point of these assertions is the boundary, not the behaviour: WP-B02 must
// not grow a colour pipeline of its own.
describe("algo 占位", () => {
  it("只暴露 NOT_IMPLEMENTED 与归属信息", () => {
    expect(NOT_IMPLEMENTED).toBe("NOT_IMPLEMENTED");
    expect(ALGO_OWNERS).toEqual({ oracle: "WP-B01", browserPipeline: "WP-B03" });
  });

  it("调用即抛，错误里带上归属工作包", () => {
    expect(() => notImplemented("CIEDE2000")).toThrow(NotImplementedError);
    expect(() => notImplemented("CIEDE2000")).toThrow(/NOT_IMPLEMENTED: CIEDE2000 归 WP-B01 \/ WP-B03/);
  });
});
