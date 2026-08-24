/**
 * The shell around the routes: the wizard comes first and stays finished, and
 * the routes other work packages own are empty rather than mocked up.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import {
  CLOUD_LABEL,
  forbidNetwork,
  installFakeCore,
  OPEN_SESSION,
  type FakeCoreOptions,
} from "./test/fakeCore";

const FIRST_RUN: FakeCoreOptions = {
  status: { ...OPEN_SESSION, wizard_completed: false },
};

async function startAtRoute(path: string, options: FakeCoreOptions = {}) {
  window.location.hash = path;
  const core = installFakeCore(options);
  render(<App />);
  await screen.findByRole("navigation", { name: "主导航" });
  return core;
}

describe("桌面壳", () => {
  it("第一次启动先走向导", async () => {
    installFakeCore(FIRST_RUN);
    render(<App />);
    expect(await screen.findByRole("heading", { name: "欢迎使用 Soul" })).toBeVisible();
    expect(screen.queryByRole("navigation", { name: "主导航" })).toBeNull();
  });

  it("向导走完之后进入概览，界面上写着全部关闭", async () => {
    installFakeCore(FIRST_RUN);
    const user = userEvent.setup();
    render(<App />);

    await screen.findByRole("heading", { name: "欢迎使用 Soul" });
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "开始使用" }));

    expect(await screen.findByTestId("closed-state")).toHaveTextContent("全部能力默认关闭");
    expect(screen.getByRole("navigation", { name: "主导航" })).toBeVisible();
  });

  /**
   * The wizard used to be a prop, which meant a real installation asked the
   * same question every launch. The answer now comes from the core, so a
   * session that says it is done goes straight to the routes.
   */
  it("核心说向导走过了，就不再问第二遍", async () => {
    window.location.hash = "#/";
    const core = installFakeCore();
    render(<App />);

    await screen.findByRole("navigation", { name: "主导航" });
    expect(screen.queryByRole("heading", { name: "欢迎使用 Soul" })).toBeNull();
    expect(core.callsTo("session_status")).toHaveLength(1);
    expect(core.callsTo("complete_wizard")).toHaveLength(0);
  });

  it("起草页有输入框，没有发送按钮", async () => {
    await startAtRoute("#/draft");

    expect(await screen.findByLabelText("原文")).toBeVisible();
    expect(screen.queryByTestId("pending-owner")).toBeNull();
    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/发送|发出|发给|回复对方/);
    }
  });

  /**
   * v0.1 promises not to execute a file move. The route now shows the plan, so
   * the absence matters more than it did while the route was empty: this
   * asserts the screen arrived *and* that nothing on it can be pressed to
   * carry the plan out.
   */
  it("文件计划页有内容了，但仍然没有任何执行按钮", async () => {
    await startAtRoute("#/files");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByRole("heading", { name: "已授权的目录" })).toBeVisible();
    expect(screen.getByTestId("no-roots")).toBeVisible();
    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/执行|应用|移动|重命名|删除/);
    }
  });

  it("人脉图页有内容了，不再是空路由", async () => {
    await startAtRoute("#/graph");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("no-people")).toBeVisible();
    expect(screen.getByTestId("graph-notice")).toHaveTextContent("工作假设，非临床结论");
  });

  it("设置页里有云端开关，且仍然尚未启用", async () => {
    const attempts = forbidNetwork();
    await startAtRoute("#/settings");

    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "false");
    expect(screen.getByTestId("cloud-state")).toHaveTextContent(CLOUD_LABEL);
    expect(attempts).toEqual([]);
  });

  it("认不出来的地址回到概览，而不是白屏", async () => {
    await startAtRoute("#/no-such-page");
    expect(screen.getByRole("heading", { name: "概览", level: 1 })).toBeVisible();
  });

  it("核心不回应时说清楚，而不是假装一切正常", async () => {
    render(<App />);
    expect(await screen.findByRole("alert")).toHaveTextContent("核心没有回应");
  });
});
