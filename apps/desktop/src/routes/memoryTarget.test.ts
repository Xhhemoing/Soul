import { describe, expect, it } from "vitest";
import { routeForHash } from "../router";
import { memoryHash, memoryTarget } from "./memoryTarget";

const ID = "0192f000-0000-7000-8000-0000000000a1";
describe("精确记忆导航目标", () => {
  it("普通页面没有自动打开目标，单个规范UUID可往返", () => {
    expect(memoryTarget("#/memory")).toBeNull();
    expect(memoryTarget(memoryHash(ID))).toBe(ID);
    expect(memoryTarget(memoryHash(ID.toUpperCase()))).toBe(ID);
    expect(routeForHash(memoryHash(ID)).id).toBe("memory");
  });
  it.each(["#/memory?", "#/memory?memory_id=", "#/memory?memory_id=not-a-uuid",
    `#/memory?memory_id=${ID}&memory_id=${ID}`, `#/memory?memory_id=${ID}&other=x`,
    `#/memory/${ID}`, `#/memory?memory_id=${ID}%2Fextra`])("拒绝无效目标 %s", (hash) => {
    expect(memoryTarget(hash)).toBe("invalid");
  });
});
