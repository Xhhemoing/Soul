/**
 * 审计: the chain played back, with nothing in it but what happened.
 *
 * The interesting assertion is the negative one. An entry has an action, a
 * decision, a reason code, ids, counts and two hashes — and no body, no prose
 * and no name, because `audit.schema.json` declares none. This checks that the
 * page renders no more than that, and that a chain that does not verify says
 * so instead of looking like one that does.
 */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { Audit } from "./Audit";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  anAuditChain,
  AUDIT_CHAIN_NOTICE,
  forbidNetwork,
  installFakeCore,
  type FakeCoreOptions,
} from "../test/fakeCore";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Audit />);
  await screen.findByRole("heading", { name: "这条链现在是什么状态" });
  return core;
}

describe("审计页", () => {
  it("链子上没有记录的时候，说清楚为什么是空的", async () => {
    await open();

    expect(screen.getByTestId("no-audit-entries")).toHaveTextContent("还没有发生过需要记下来的事");
    expect(screen.getByTestId("audit-notice")).toHaveTextContent(AUDIT_CHAIN_NOTICE);
  });

  it("回放按顺序列出动作、结论和前后哈希", async () => {
    await open({ audit: () => anAuditChain() });

    expect(screen.getByTestId("audit-verified")).toHaveTextContent("2 条记录，逐条对上了");
    const first = screen.getByTestId("audit-entry-1");
    expect(first).toHaveTextContent("写入一条记忆");
    expect(first).toHaveTextContent("放行");
    expect(screen.getByTestId("audit-link-2")).toHaveTextContent("111111111111 → 222222222222");
  });

  /**
   * The chain records that something happened; it does not record what. An
   * entry has no field that could hold prose, so the only way a body could
   * appear on this page is if the component invented one.
   */
  it("回放里没有正文、没有姓名，只有编号和计数", async () => {
    await open({ audit: () => anAuditChain() });

    const entries = screen.getByTestId("audit-entries");
    expect(entries).toHaveTextContent("涉及 1 个编号");
    expect(entries).toHaveTextContent("1 项");
    // The subject is a bare identifier, and it is counted rather than listed:
    // an id on screen is a thing a reader could look up, and this page is not
    // where anything gets looked up.
    expect(entries.textContent ?? "").not.toContain("0192f000-0000-7000-8000-0000000000a1");
    expect(entries.textContent ?? "").not.toMatch(/搬家|钥匙|厨房/);
  });

  /** A break has to be visible, and visible at the line it happens on. */
  it("链子对不上的时候照实说，并且指出断在哪一条", async () => {
    await open({
      audit: () =>
        anAuditChain({
          verified: false,
          verification_problem: "第 2 条的 prev_hash 和第 1 条对不上",
          entries: anAuditChain().entries.map((entry) =>
            entry.seq === 2 ? { ...entry, follows_previous: false } : entry,
          ),
        }),
    });

    expect(screen.getByTestId("audit-verified")).toHaveTextContent("链子对不上");
    expect(screen.getByTestId("audit-verified")).toHaveTextContent("prev_hash");
    expect(screen.getByTestId("audit-link-2")).toHaveTextContent("这里接不上上一条");
  });

  it("渲染出来的回放里没有一个诊断词或量表词", async () => {
    const terms = diagnosticTerms();
    expect(terms.length).toBeGreaterThan(50);

    await open({ audit: () => anAuditChain() });

    expect(denylistHits(renderedText(), terms)).toEqual([]);
  });

  it("这一页只读：没有按钮，也没有碰过网络", async () => {
    const attempts = forbidNetwork();
    await open({ audit: () => anAuditChain() });

    expect(screen.queryAllByRole("button")).toEqual([]);
    expect(attempts).toEqual([]);
  });

  it("读不到库的时候给出理由码，而不是一条空链", async () => {
    installFakeCore({
      audit: () => {
        throw {
          reason_code: "STORE_UNAVAILABLE",
          explanation: "数据库这次没有打开，所以回放不了审计链。",
        };
      },
    });
    render(<Audit />);

    expect(await screen.findByTestId("audit-refusal-code")).toHaveTextContent("STORE_UNAVAILABLE");
    expect(screen.queryByTestId("audit-entries")).toBeNull();
  });
});
