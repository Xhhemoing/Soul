/**
 * The collection page: the only switch in v0.1 that turns a capability on.
 *
 * `soul-collect` proves that the pipeline collects duration and nothing else,
 * and `soulcore/tests/session_collect.rs` proves the session gate. What only
 * this side can check is the thing that was missing until this route existed —
 * that an installed Soul has a way to reach the gate at all — and that the page
 * reaching it never puts an application name on screen.
 *
 * That last one is checked against the rendered page rather than against the
 * fixture, because `CollectStatus` has no field an application name could
 * arrive in. A component that showed one would have had to compose it, and
 * composing is exactly what the assertions below look for.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { Collect } from "./Collect";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  aCollectStatus,
  COLLECT_NOT_OBSERVING_NOTICE,
  COLLECT_OFF_NOTICE,
  COLLECT_RUNNING_NOTICE,
  forbidNetwork,
  installFakeCore,
  type FakeCoreOptions,
} from "../test/fakeCore";

/** An application somebody was in front of, for the page to not know about. */
const AN_APP = "outlook.exe";

function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  const user = userEvent.setup();
  render(<Collect />);
  return { core, user };
}

describe("采集页", () => {
  it("说清楚采的是前台应用的时长，不采窗口标题", async () => {
    open();

    expect(await screen.findByTestId("collect-state")).toBeVisible();
    const text = renderedText();
    expect(text).toContain("前台");
    expect(text).toContain("时长");
    expect(text).toContain("窗口标题");
    expect(screen.getByTestId("collect-duration-only")).toHaveTextContent("窗口标题不记");
    expect(screen.getByTestId("collect-restart")).toHaveTextContent("采集还是关的");
  });

  it("默认是关的，两个按钮都在", async () => {
    open();

    expect(await screen.findByTestId("collect-state")).toHaveTextContent("没有在采集");
    expect(screen.getByTestId("collect-consent")).toHaveTextContent("没有给出");
    expect(screen.getByTestId("collect-running")).toHaveTextContent("没有在跑");
    expect(screen.getByTestId("collect-notice")).toHaveTextContent(COLLECT_OFF_NOTICE);
    expect(screen.getByRole("button", { name: "开始采集" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "停止采集" })).toBeDisabled();
  });

  it("按开始采集，屏幕上变成正在采集；按停止采集，又回到关", async () => {
    const { core, user } = open();
    await screen.findByTestId("collect-state");

    await user.click(screen.getByRole("button", { name: "开始采集" }));

    expect(await screen.findByTestId("collect-state")).toHaveTextContent("正在采集");
    expect(screen.getByTestId("collect-running")).toHaveTextContent("在跑");
    expect(screen.getByTestId("collect-notice")).toHaveTextContent(COLLECT_RUNNING_NOTICE);
    expect(screen.getByRole("button", { name: "开始采集" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "停止采集" }));

    expect(await screen.findByTestId("collect-state")).toHaveTextContent("没有在采集");
    expect(screen.getByTestId("collect-consent")).toHaveTextContent("没有给出");
    expect(core.callsTo("grant_collect_consent")).toHaveLength(1);
    expect(core.callsTo("revoke_collect_consent")).toHaveLength(1);
  });

  /**
   * A build with no foreground source records the consent and watches nothing.
   * The page has to say both halves: reporting "正在采集" on a Linux box would
   * be the one failure mode `platform_source` exists to refuse.
   */
  it("没有前台来源的时候，同意归同意，屏幕上不说在采", async () => {
    const { user } = open({ collect: aCollectStatus({ source: "unsupported" }) });
    await screen.findByTestId("collect-state");

    await user.click(screen.getByRole("button", { name: "开始采集" }));

    expect(await screen.findByTestId("collect-state")).toHaveTextContent("已经同意，但没有在采");
    expect(screen.getByTestId("collect-consent")).toHaveTextContent("已给出");
    expect(screen.getByTestId("collect-running")).toHaveTextContent("没有在跑");
    expect(screen.getByTestId("collect-notice")).toHaveTextContent(COLLECT_NOT_OBSERVING_NOTICE);
    expect(screen.getByTestId("collect-source")).toHaveTextContent("没有前台来源");
  });

  /**
   * Counts are the whole vocabulary of this page. Seven events become nine and
   * the only thing that changes on screen is a number: no name, no path, and
   * nothing that could be read as either.
   */
  it("屏幕上是条数，不是应用名", async () => {
    const { user } = open({
      collect: aCollectStatus({ events_collected: 7 }),
      granting: (current) => ({
        ...current,
        consent_granted: true,
        collector_running: true,
        events_collected: 9,
        notice: COLLECT_RUNNING_NOTICE,
      }),
    });
    await screen.findByTestId("collect-state");

    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 7 条前台记录");
    await user.click(screen.getByRole("button", { name: "开始采集" }));
    expect(await screen.findByTestId("collect-count")).toHaveTextContent("已经有 9 条前台记录");

    const text = renderedText();
    expect(text).not.toContain(AN_APP);
    expect(text).not.toMatch(/\.exe/);
    expect(text).not.toMatch(/[A-Za-z]:\\/);
  });

  it("页面上没有发送、导出或者执行的按钮", async () => {
    open();
    await screen.findByTestId("collect-state");

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/发送|上传|导出|执行|同步/);
    }
    expect(renderedText()).not.toMatch(/上传|同步到/);
  });

  it("库没打开的时候给的是理由码和核心那句话", async () => {
    const { user } = open({
      granting: () => {
        throw {
          reason_code: "ROUTINE",
          explanation: "这台机器上的加密库没有打开，所以要读库的页面暂时没有内容可显示。",
        };
      },
    });
    await screen.findByTestId("collect-state");

    await user.click(screen.getByRole("button", { name: "开始采集" }));

    expect(await screen.findByTestId("collect-refusal-code")).toHaveTextContent("ROUTINE");
    expect(screen.getByRole("alert")).toHaveTextContent("加密库没有打开");
    expect(screen.getByTestId("collect-state")).toHaveTextContent("没有在采集");
  });

  it("渲染出来的页面里没有一个诊断词或量表词", async () => {
    const { user } = open();
    await screen.findByTestId("collect-state");
    await user.click(screen.getByRole("button", { name: "开始采集" }));
    await screen.findByTestId("collect-notice");

    expect(denylistHits(renderedText(), diagnosticTerms())).toEqual([]);
  });

  it("开关采集的全程没有碰过网络", async () => {
    const attempts = forbidNetwork();
    const { user } = open();
    await screen.findByTestId("collect-state");
    await user.click(screen.getByRole("button", { name: "开始采集" }));
    await user.click(screen.getByRole("button", { name: "停止采集" }));
    await screen.findByTestId("collect-notice");

    expect(attempts).toEqual([]);
  });
});
