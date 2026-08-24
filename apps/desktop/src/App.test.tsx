/**
 * The shell around the routes: the wizard comes first and stays finished, and
 * the routes other work packages own are empty rather than mocked up.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import { ROUTES } from "./router";
import {
  CLOUD_LABEL,
  COLLECT_RUNNING,
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
    await user.click(screen.getByRole("button", { name: "下一步" }));
    await screen.findByTestId("wizard-questions");
    await user.click(screen.getByRole("button", { name: "一题都不答，直接开始" }));

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

  /**
   * The two formats PRODUCT_LOCK names have to be reachable from the installed
   * app, not only from the headless smoke run. This is the route that makes
   * that true, so the check is that it arrives with a file input on it.
   */
  it("导入页有内容了，两种格式都在，选文件的框也在", async () => {
    await startAtRoute("#/import");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByText(/soul-import-v1 JSONL/)).toBeVisible();
    expect(screen.getByText(/Telegram Desktop 的 result.json/)).toBeVisible();
    expect(screen.getByLabelText("选择文件")).toHaveAttribute("type", "file");
  });

  it("人脉图页有内容了，不再是空路由", async () => {
    await startAtRoute("#/graph");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("no-people")).toBeVisible();
    expect(screen.getByTestId("graph-notice")).toHaveTextContent("工作假设，非临床结论");
  });

  it("灵魂档案页有内容了，不再是空路由", async () => {
    await startAtRoute("#/profile");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("profile-reading")).toBeVisible();
    expect(screen.getByTestId("profile-notice")).toHaveTextContent("工作假设，非临床结论");
  });

  it("自传记忆页有内容了，不再是空路由", async () => {
    await startAtRoute("#/memory");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("no-memories")).toBeVisible();
    expect(screen.getByTestId("forget-notice")).toHaveTextContent("不写任何文件");
  });

  it("研究预览页有内容了，不再是空路由", async () => {
    await startAtRoute("#/research");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("research-on-screen-only")).toHaveTextContent("没有落盘");
    expect(screen.getByTestId("research-third-party")).toHaveTextContent("别人的数据 0 行");
  });

  /**
   * The route this work package exists for. Until it landed, an installed Soul
   * had no way to reach the consent ledger at all, so the check is the plain
   * one: the page arrives, and the switch that turns collection on is on it.
   */
  it("采集页有内容了，而且开关就在这一页上", async () => {
    await startAtRoute("#/collect");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("collect-state")).toBeVisible();
    expect(screen.getByRole("button", { name: "开始采集" })).toBeVisible();
    expect(screen.getByRole("button", { name: "停止采集" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "采集", level: 1 })).toBeVisible();
  });

  /**
   * 设置 is the cloud page, and the cloud is a capability this build does not
   * have. Collection is one it does, gated by a ledger rather than by a
   * configuration field, and a toggle for it sitting beside the cloud switch
   * would suggest the two are the same kind of thing.
   */
  it("设置页上没有采集开关", async () => {
    await startAtRoute("#/settings");

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/采集/);
    }
    expect(screen.queryByTestId("collect-state")).toBeNull();
    expect(screen.getAllByRole("switch")).toHaveLength(1);
  });

  /**
   * 概览 used to read `ConfigSnapshot.collect_enabled`, which is false for the
   * whole life of the process however much is being collected: nothing writes
   * that field at runtime and `config.json` has nowhere to keep it. The
   * fixture below is exactly that disagreement — a snapshot saying everything
   * is closed, and a ledger with a collector running — and the line has to
   * follow the ledger.
   */
  it("概览上的采集那一行读的是同意账本，不是配置里的旧字段", async () => {
    await startAtRoute("#/", { collect: COLLECT_RUNNING });

    const fact = await screen.findByTestId("collect-fact");
    expect(fact).toHaveTextContent("正在采集");
    expect(screen.getByTestId("closed-state")).toHaveTextContent("全部能力默认关闭");
    expect(within(fact).getByRole("link", { name: "采集" })).toHaveAttribute("href", "#/collect");
  });

  it("概览关着的时候和采集页用同一句话", async () => {
    await startAtRoute("#/");

    expect(await screen.findByTestId("collect-fact")).toHaveTextContent("没有在采集");
  });

  it("审计页有内容了，不再是空路由", async () => {
    await startAtRoute("#/audit");

    expect(screen.queryByTestId("pending-owner")).toBeNull();
    expect(await screen.findByTestId("no-audit-entries")).toBeVisible();
    expect(screen.getByTestId("audit-notice")).toHaveTextContent("不记内容");
  });

  /**
   * `router.tsx` has no `ownedBy` left, so `Pending` never renders. It is kept
   * rather than deleted — the next route added before its view exists should
   * say whose it is instead of looking like a feature — and this is the check
   * that no route is quietly relying on it today.
   */
  it("没有一个路由还是空的", async () => {
    for (const route of ROUTES) {
      expect(route.ownedBy).toBeNull();
    }
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
