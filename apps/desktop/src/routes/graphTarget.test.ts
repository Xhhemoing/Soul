import { describe, expect, it } from "vitest";
import { routeForHash } from "../router";
import { aPeopleGraph } from "../test/fakeCore";
import { graphTargetHash, MAX_GRAPH_TARGET_HASH_CHARS, MAX_GRAPH_TARGET_REFERENCES, parseGraphTarget, resolveGraphTarget } from "./graphTarget";

const graph = aPeopleGraph();
const contact = graph.people[1]!.contact_id;
const evidence = graph.ties[0]!.evidence.map((row) => row.evidence_id);
const id = (value: number): string => `0192f000-0000-7000-8000-${value.toString(16).padStart(12, "0")}`;

describe("本地关系与证据元数据目标", () => {
  it("规范化UUID大小写并对builder引用去重，parser拒绝重复歧义", () => {
    const hash = graphTargetHash(contact.toUpperCase(), [evidence[0]!.toUpperCase(), evidence[0]!]);
    expect(hash).not.toBeNull(); expect(parseGraphTarget(hash!)).toEqual({ kind: "selection", contactId: contact, evidenceIds: [evidence[0]] });
    expect(parseGraphTarget(`${hash}&evidence=${evidence[0]!.toUpperCase()}`)).toEqual({ kind: "invalid" });
  });
  it.each([
    "#/graph?", `#/graph?contact=${contact}`, `#/graph?evidence=${evidence[0]}`,
    `#/graph?contact=${contact}&contact=${contact}&evidence=${evidence[0]}`,
    `#/graph?contact=${contact}&evidence=not-a-uuid`, `#/graph?contact=unknown&evidence=${evidence[0]}`,
    `#/graph?contact=${contact}&evidence=${evidence[0]}&instruction=write`,
    `#/graph?contact=${contact}&evidence=${evidence[0]}&path=%2Fprivate`,
  ])("不接受不完整或扩权参数：%s", (hash) => { expect(parseGraphTarget(hash)).toEqual({ kind: "invalid" }); });
  it("引用和raw hash都有上限，整份超限不截断成另一份目标", () => {
    const refs = Array.from({ length: MAX_GRAPH_TARGET_REFERENCES }, (_, index) => id(index + 1));
    expect(graphTargetHash(contact, refs)).not.toBeNull();
    expect(graphTargetHash(contact, [...refs, id(100)])).toBeNull();
    expect(parseGraphTarget(`#/graph?contact=${contact}&${refs.map((ref) => `evidence=${ref}`).join("&")}&evidence=${id(100)}`)).toEqual({ kind: "invalid" });
    expect(parseGraphTarget(`#/graph?${"x".repeat(MAX_GRAPH_TARGET_HASH_CHARS)}`)).toEqual({ kind: "invalid" });
  });
  it("只扩展Graph查询，其他路径仍精确匹配", () => {
    expect(routeForHash(graphTargetHash(contact, evidence)!).id).toBe("graph");
    expect(routeForHash("#/graph?invalid=true").id).toBe("graph");
    for (const hash of ["#/graph/other", "#/graphical?contact=x", "#/memory?contact=x", "#/import?evidence=x"]) expect(routeForHash(hash).id).toBe("home");
    expect(parseGraphTarget("#/graph")).toEqual({ kind: "none" });
  });
  it("仅当前self/contact唯一边包含全部指定引用时定位，允许明确的引用子集", () => {
    const target = parseGraphTarget(graphTargetHash(contact, [evidence[0]!])!);
    const result = resolveGraphTarget(graph, target);
    expect(result.kind).toBe("ready");
    if (result.kind === "ready") { expect(result.tie.relationship_id).toBe(graph.ties[0]!.relationship_id); expect(result.evidence.map((row) => row.evidence_id)).toEqual([evidence[0]]); }
    expect(resolveGraphTarget({ ...graph, ties: [{ ...graph.ties[0]!, evidence: graph.ties[0]!.evidence.slice(1) }] }, target).kind).toBe("unavailable");
    expect(resolveGraphTarget({ ...graph, ties: [graph.ties[0]!, { ...graph.ties[0]!, relationship_id: id(999) }] }, target).kind).toBe("unavailable");
  });
  it("未知或遗忘联系人不改选其它边，缺失self节点也拒绝", () => {
    const target = parseGraphTarget(graphTargetHash(contact, evidence)!);
    expect(resolveGraphTarget({ ...graph, people: graph.people.map((person) => person.contact_id === contact ? { ...person, forgotten: true } : person) }, target).kind).toBe("unavailable");
    expect(resolveGraphTarget({ ...graph, people: graph.people.filter((person) => !person.is_you) }, target).kind).toBe("unavailable");
    expect(resolveGraphTarget(graph, parseGraphTarget(graphTargetHash(id(404), evidence)!)).kind).toBe("unavailable");
  });
});
