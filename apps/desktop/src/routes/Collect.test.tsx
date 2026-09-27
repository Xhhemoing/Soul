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

import { act, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Collect } from "./Collect";
import type { CollectStatus } from "../core";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  aCollectStatus,
  COLLECT_NOT_OBSERVING_NOTICE,
  COLLECT_OFF_NOTICE,
  COLLECT_RUNNING_NOTICE,
  forbidNetwork,
  installFakeCore,
  type FakeCore,
  type FakeCoreOptions,
} from "../test/fakeCore";

/** An application somebody was in front of, for the page to not know about. */
const AN_APP = "outlook.exe";

/** The page's own reread rate, which is the collector's poll interval. */
const A_REREAD = 1000;

function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  const user = userEvent.setup();
  render(<Collect />);
  return { core, user };
}

/**
 * Mount with the clock in the test's hands.
 *
 * The page keeps a timer of its own now, so a test that let the real clock run
 * would be asserting on how quickly it reached its own assertions. Clicks go
 * through `fireEvent` rather than `userEvent` because user-event schedules its
 * waits on the very clock these tests are holding still.
 */
function openOnAHeldClock(options: FakeCoreOptions = {}): FakeCore {
  vi.useFakeTimers();
  const core = installFakeCore(options);
  render(<Collect />);
  return core;
}

/** Let every answer in flight land, and let the page's timer run `ms`. */
async function settle(ms = 0): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

/**
 * A core that counts one more event every time a running collector is read.
 *
 * The real one recounts the table on each `collect_status`, so a page sitting
 * still in front of a running collector sees a number move without having
 * pressed anything. A double answering a constant could not tell a page that
 * rereads from one frozen on mount.
 */
function bumpWhileRunning(current: CollectStatus): CollectStatus {
  return current.collector_running
    ? { ...current, events_collected: (current.events_collected ?? 0) + 1 }
    : current;
}

afterEach(() => {
  vi.useRealTimers();
});

describe("采集页", () => {
  it.each(["success", "failure"] as const)("停止采集后忽略先前状态读取的 %s", async (outcome) => {
    let resolve!: (status: CollectStatus) => void;
    let reject!: (reason: unknown) => void;
    const pending = new Promise<CollectStatus>((yes, no) => { resolve = yes; reject = no; });
    const running = aCollectStatus({ consent_granted: true, collector_running: true, events_collected: 12 });
    let reads = 0;
    mockIPC((command) => {
      if (command === "collect_status") return ++reads === 1 ? running : pending;
      if (command === "revoke_collect_consent") return aCollectStatus({ events_collected: 13 });
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Collect />);
    await screen.findByTestId("collect-state");
    await user.click(screen.getByRole("button", { name: "看现在的条数" }));
    await user.click(screen.getByRole("button", { name: "停止采集" }));
    expect(screen.getByTestId("collect-state")).toHaveTextContent("没有在采集");

    await act(async () => {
      if (outcome === "success") resolve(running);
      else reject({ reason_code: "ROUTINE", explanation: "先前读取失败" });
    });
    expect(screen.getByTestId("collect-state")).toHaveTextContent("没有在采集");
    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 13 条");
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("读取未返回时不重复发送状态请求", async () => {
    let resolve!: (status: CollectStatus) => void;
    const pending = new Promise<CollectStatus>((yes) => { resolve = yes; });
    let reads = 0;
    mockIPC((command) => {
      if (command === "collect_status") return ++reads === 1 ? aCollectStatus() : pending;
      throw new Error(`unexpected command: ${command}`);
    });
    const user = userEvent.setup();
    render(<Collect />);
    await screen.findByTestId("collect-state");
    const refresh = screen.getByRole("button", { name: "看现在的条数" });
    await user.click(refresh);
    await user.click(refresh);
    expect(reads).toBe(2);
    await act(async () => resolve(aCollectStatus({ events_collected: 9 })));
    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 9 条");
  });

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

  /**
   * The bug this page had: the count was read once on mount and once per
   * button, so pressing 开始采集 and then watching — the thing a person
   * actually does — showed a frozen number while the collector wrote events
   * behind it. The author manual said 刷新这一页, which on an installed Soul
   * meant leaving the route and coming back.
   */
  it("采着的时候条数自己往上走，不用离开这一页再回来", async () => {
    openOnAHeldClock({
      collect: aCollectStatus({ events_collected: 12 }),
      rereading: bumpWhileRunning,
    });
    await settle();

    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 12 条前台记录");

    fireEvent.click(screen.getByRole("button", { name: "开始采集" }));
    await settle();

    expect(screen.getByTestId("collect-state")).toHaveTextContent("正在采集");
    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 12 条前台记录");

    await settle(A_REREAD);
    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 13 条前台记录");

    await settle(A_REREAD);
    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 14 条前台记录");

    const text = renderedText();
    expect(text).not.toContain(AN_APP);
    expect(text).not.toMatch(/\.exe/);
    expect(text).not.toMatch(/[A-Za-z]:\\/);
  });

  it("按「看现在的条数」，同一页上就换成新的条数", async () => {
    let events = 7;
    openOnAHeldClock({
      collect: aCollectStatus({ events_collected: events }),
      rereading: (current) => ({ ...current, events_collected: events }),
    });
    await settle();

    fireEvent.click(screen.getByRole("button", { name: "开始采集" }));
    await settle();
    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 7 条前台记录");

    events = 31;
    fireEvent.click(screen.getByRole("button", { name: "看现在的条数" }));
    await settle();

    expect(screen.getByTestId("collect-count")).toHaveTextContent("已经有 31 条前台记录");
  });

  /**
   * The check the manual asks for after 停止采集 is that the count stopped
   * climbing, and that check needs a reread the user can ask for without
   * leaving. The timer is not it: a stopped collector cannot move the number,
   * so a page still polling then would be spending IPC to be told the same
   * thing.
   */
  it("停下之后不用离开这一页，按一下就能再问一次核心", async () => {
    const core = openOnAHeldClock();
    await settle();

    fireEvent.click(screen.getByRole("button", { name: "开始采集" }));
    await settle();
    fireEvent.click(screen.getByRole("button", { name: "停止采集" }));
    await settle();
    expect(screen.getByTestId("collect-state")).toHaveTextContent("没有在采集");

    const asked = core.callsTo("collect_status").length;
    await settle(30 * A_REREAD);
    expect(core.callsTo("collect_status")).toHaveLength(asked);

    fireEvent.click(screen.getByRole("button", { name: "看现在的条数" }));
    await settle();

    expect(core.callsTo("collect_status")).toHaveLength(asked + 1);
  });

  it("没有在采的时候，这一页不会自己一遍遍去问核心", async () => {
    const core = openOnAHeldClock();
    await settle();

    expect(core.callsTo("collect_status")).toHaveLength(1);

    await settle(30 * A_REREAD);

    expect(core.callsTo("collect_status")).toHaveLength(1);
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
