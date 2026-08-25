/**
 * 设置: the page that finally reaches the endpoint the core has always had.
 *
 * `soulcore/tests/session_e1.rs` proves the session gate — the guard is
 * re-pointed, the address never reaches `config.json`, and filling it in
 * contacts nothing — and `src-tauri/tests/ipc_roundtrip.rs` proves the two
 * commands are registered. What only this side can check is the thing that was
 * missing until this form existed: that an installed Soul has a way to reach
 * the gate at all, and that the page reaching it never puts the address back on
 * screen from anywhere but the box the user is typing in.
 *
 * The snapshot is held by the harness rather than by the page, because that is
 * how `App` holds it: 概览 renders 语言模型端点 off the same value, and a page
 * that kept its own copy would let the two disagree.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { Settings } from "./Settings";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  CLOSED_SNAPSHOT,
  ENDPOINT_UNPARSABLE_NOTICE,
  forbidNetwork,
  installFakeCore,
  LLM_ENDPOINT_SESSION_ONLY_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";
import type { ConfigSnapshot } from "../core";

/** A loopback model server, the way somebody running one would name it. */
const AN_ADDRESS = "http://127.0.0.1:11434/v1";

/**
 * `App`, reduced to the part this page depends on: it owns the snapshot and
 * hands the page whatever the core last answered with.
 */
function Harness({ onSnapshot }: { onSnapshot?: (next: ConfigSnapshot) => void }) {
  const [snapshot, setSnapshot] = useState<ConfigSnapshot>(CLOSED_SNAPSHOT);
  return (
    <Settings
      snapshot={snapshot}
      onSnapshot={(next) => {
        setSnapshot(next);
        onSnapshot?.(next);
      }}
    />
  );
}

function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  const user = userEvent.setup();
  const seen = vi.fn();
  render(<Harness onSnapshot={seen} />);
  return { core, user, seen };
}

async function type(user: ReturnType<typeof userEvent.setup>, address: string): Promise<void> {
  await user.type(screen.getByLabelText("端点地址"), address);
}

describe("设置页的端点表单", () => {
  it("默认未填写，两个按钮都在，保存要先有地址", () => {
    open();

    expect(screen.getByTestId("endpoint-state")).toHaveTextContent("未填写");
    expect(screen.getByTestId("endpoint-notice")).toHaveTextContent(
      LLM_ENDPOINT_SESSION_ONLY_NOTICE,
    );
    expect(screen.getByRole("button", { name: "保存端点" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "清除端点" })).toBeDisabled();
  });

  it("填地址按保存，屏幕上变成已填写；按清除，又回到未填写", async () => {
    const { core, user, seen } = open();

    await type(user, AN_ADDRESS);
    await user.click(screen.getByRole("button", { name: "保存端点" }));

    expect(await screen.findByTestId("endpoint-state")).toHaveTextContent("已填写");
    expect(core.callsTo("set_user_endpoint")).toEqual([
      { cmd: "set_user_endpoint", payload: { url: AN_ADDRESS } },
    ]);
    // 概览 is drawn from the value the core answered with, so the page has to
    // hand it up rather than keep the change to itself.
    expect(seen).toHaveBeenCalledWith(
      expect.objectContaining({ llm_endpoint_configured: true, fully_closed: false }),
    );

    await user.click(screen.getByRole("button", { name: "清除端点" }));

    expect(await screen.findByTestId("endpoint-state")).toHaveTextContent("未填写");
    expect(core.callsTo("clear_user_endpoint")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "清除端点" })).toBeDisabled();
  });

  /**
   * The address is write-only. `ConfigSnapshot` has no field it could come back
   * in, so the only place it can appear is the box the user typed it into —
   * and after 清除 not even there.
   */
  it("保存之后，地址只在输入框里，不在页面的说明里", async () => {
    const { user } = open();

    await type(user, AN_ADDRESS);
    await user.click(screen.getByRole("button", { name: "保存端点" }));
    await screen.findByTestId("endpoint-state");

    expect(renderedText()).not.toContain("11434");
    await user.click(screen.getByRole("button", { name: "清除端点" }));
    expect(await screen.findByLabelText("端点地址")).toHaveValue("");
  });

  it("地址不是地址的时候，屏幕上是核心的理由码和那句话", async () => {
    const { user } = open();

    await type(user, "我的模型");
    await user.click(screen.getByRole("button", { name: "保存端点" }));

    expect(await screen.findByTestId("endpoint-refusal-code")).toHaveTextContent(
      "EGRESS_TARGET_UNPARSABLE",
    );
    expect(screen.getByRole("alert")).toHaveTextContent(ENDPOINT_UNPARSABLE_NOTICE);
    expect(screen.getByTestId("endpoint-state")).toHaveTextContent("未填写");
  });

  it("页面上没有发送、上传或者执行的按钮", () => {
    open();

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/发送|上传|导出|执行|同步/);
    }
    expect(renderedText()).not.toMatch(/上传|同步到/);
  });

  it("渲染出来的页面里没有一个诊断词或量表词", async () => {
    const { user } = open();
    await type(user, AN_ADDRESS);
    await user.click(screen.getByRole("button", { name: "保存端点" }));
    await screen.findByTestId("endpoint-state");

    expect(denylistHits(renderedText(), diagnosticTerms())).toEqual([]);
  });

  /**
   * The whole point of the form: naming an address is not contacting it. The
   * core proves that against a real loopback server; here the claim is about
   * this page, which has no client of its own and must not grow one.
   */
  it("填写与清除的全程没有碰过网络", async () => {
    const attempts = forbidNetwork();
    const { user } = open();

    await type(user, AN_ADDRESS);
    await user.click(screen.getByRole("button", { name: "保存端点" }));
    await screen.findByTestId("endpoint-state");
    await user.click(screen.getByRole("button", { name: "清除端点" }));
    await screen.findByTestId("endpoint-state");

    expect(attempts).toEqual([]);
  });
});

describe("设置页的其它部分", () => {
  it("云端开关和三条出网说明还在", () => {
    open();

    expect(screen.getByTestId("cloud-state")).toHaveTextContent("尚未启用");
    const text = renderedText();
    expect(text).toContain("不自动更新");
    expect(text).toContain("不加载任何远程页面");
    expect(text).toContain("不要求管理员权限");
  });

  /**
   * 出网 has to name both of the paths that reach the endpoint. Saying 只在你按下
   * 生成的时候 reads as "the drafting page and nowhere else", and 人脉图 上
   * 「看这个人的摘要」 is one click and one request with no confirmation screen
   * in between — the surprise lands on whoever pointed Soul at a metered
   * address.
   */
  it("出网那一条把摘要那条路也说出来了", () => {
    open();

    const when = screen.getByTestId("egress-endpoint-when").textContent ?? "";
    expect(when).toContain("生成");
    expect(when).toMatch(/摘要|人脉图/);
    expect(when).not.toContain("只在你按下生成的时候");
  });

  /** 采集 lives on its own page, and this one must not grow a second switch. */
  it("这一页上没有采集开关", () => {
    open();

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/采集/);
    }
  });
});
