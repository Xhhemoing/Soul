/**
 * The settings page, which in this work package is the only screen that can
 * write anything.
 *
 * Two claims are asserted here and nowhere else on the TypeScript side. The
 * sentence about the database key is the core's constant rendered verbatim —
 * `contract.test.ts` checks that the constant in `fakeCore` is still the one
 * in `shell.rs`, and this checks that the constant is what reaches the screen.
 * And the authorisation control is a text box and a button: no directory
 * picker, no filesystem plugin, and no opinion in this file about whether a
 * path is any good.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import type { ConfigSnapshot } from "../core";
import {
  CLOSED_SNAPSHOT,
  forbidNetwork,
  installFakeCore,
  KEY_PROTECTION,
  REFUSED_ROOT,
  REFUSED_ROOT_MESSAGE,
} from "../test/fakeCore";
import { Settings } from "./Settings";

/**
 * The page is controlled: it renders the snapshot it is handed and reports the
 * one the core answered with. `App` holds that state in the real shell, so the
 * test holds it here rather than letting the page keep a copy of its own.
 */
function Harness(): React.JSX.Element {
  const [snapshot, setSnapshot] = useState<ConfigSnapshot>(CLOSED_SNAPSHOT);
  return <Settings snapshot={snapshot} onSnapshot={setSnapshot} />;
}

async function authorize(path: string): Promise<void> {
  const user = userEvent.setup();
  await user.clear(screen.getByRole("textbox", { name: "目录的完整路径" }));
  await user.type(screen.getByRole("textbox", { name: "目录的完整路径" }), path);
  await user.click(screen.getByRole("button", { name: "授权这个目录" }));
}

describe("设置页", () => {
  it("密钥说明是核心给的那句话，一个字都没改", async () => {
    installFakeCore();
    render(<Harness />);

    const line = await screen.findByTestId("key-protection");
    expect(line.textContent).toBe(KEY_PROTECTION);
  });

  it("授权入口只有一个文本框和一个按钮", () => {
    installFakeCore();
    render(<Harness />);

    const panel = screen.getByRole("region", { name: "授权目录" });
    expect(within(panel).getAllByRole("textbox")).toHaveLength(1);
    expect(within(panel).getByRole("button", { name: "授权这个目录" })).toBeVisible();
    expect(within(panel).queryByRole("dialog")).toBeNull();
    expect(within(panel).queryByRole("combobox")).toBeNull();
  });

  it("授权成功之后，列表里是核心解析出来的那条路径", async () => {
    const attempts = forbidNetwork();
    const core = installFakeCore();
    render(<Harness />);
    await screen.findByText("还没有授权任何目录。");

    await authorize("/tmp/soul-fixture");

    const list = screen.getByTestId("authorized-roots");
    expect(await within(list).findByText("/tmp/soul-fixture")).toBeVisible();
    expect(core.callsTo("authorize_root")).toHaveLength(1);
    expect(core.callsTo("authorize_root")[0]?.payload).toEqual({ path: "/tmp/soul-fixture" });
    expect(attempts).toEqual([]);
  });

  /**
   * The count on screen is the one the core answered with, not the length of a
   * list this page appended to. A page that counted its own inputs would agree
   * with the core right up until the core refused something.
   */
  it("计数来自核心返回的快照", async () => {
    installFakeCore();
    render(<Harness />);
    expect(screen.getByTestId("authorized-root-count")).toHaveTextContent("已授权 0 个目录");

    await authorize("/tmp/soul-fixture");

    expect(await screen.findByText("/tmp/soul-fixture")).toBeVisible();
    expect(screen.getByTestId("authorized-root-count")).toHaveTextContent("已授权 1 个目录");
  });

  it("非法路径显示核心的原话，而不是界面自己编一句", async () => {
    installFakeCore();
    render(<Harness />);

    await authorize(REFUSED_ROOT);

    const refusal = await screen.findByRole("alert");
    expect(refusal).toHaveTextContent(REFUSED_ROOT_MESSAGE);
    expect(screen.getByTestId("authorized-roots")).toHaveTextContent("还没有授权任何目录。");
  });

  it("如实写着这份清单只在本次会话里有效", () => {
    installFakeCore();
    render(<Harness />);

    const note = screen.getByTestId("roots-are-session-only");
    expect(note).toHaveTextContent("本次会话有效，尚无持久化");
  });

  it("整页没有任何一次网络调用", async () => {
    const attempts = forbidNetwork();
    installFakeCore();
    render(<Harness />);

    await authorize("/tmp/soul-fixture");
    await screen.findByText("/tmp/soul-fixture");

    expect(attempts).toEqual([]);
  });
});
