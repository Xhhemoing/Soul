/**
 * The people screen: counts and evidence, no names, and nothing a medical
 * product would say.
 *
 * `soul-draft` already tests that a summary cites evidence and passes the
 * denylist. What only this side can check is that the screen renders the
 * core's sentences rather than composing its own, and that a graph with
 * somebody in it still has no name on it.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { Graph } from "./Graph";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  aPeopleGraph,
  aPersonSummary,
  forbidNetwork,
  installFakeCore,
  NOT_A_BAND_NOTICE,
  WORKING_HYPOTHESIS_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";

/** The tie `aPeopleGraph` carries, which every correction below is about. */
const TIE_ID = "0192f000-0000-7000-8000-00000000000a";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Graph />);
  await screen.findByRole("heading", { name: /这台机器上看得见的人/ });
  return core;
}

describe("人脉图页", () => {
  it("库里没有人的时候，说清楚为什么是空的", async () => {
    await open();

    expect(screen.getByTestId("no-people")).toHaveTextContent("还没有可以显示的人");
    expect(screen.queryByTestId("ties-list")).toBeNull();
    expect(screen.getByTestId("graph-notice")).toHaveTextContent(WORKING_HYPOTHESIS_NOTICE);
  });

  it("有人的时候，列出计数、关系形状和背后的证据条数", async () => {
    await open({ graph: aPeopleGraph() });

    const people = screen.getByTestId("people-list");
    expect(within(people).getByText("bbbbbbbb")).toBeVisible();
    expect(people).toHaveTextContent("往来 6 次");

    const ties = screen.getByTestId("ties-list");
    expect(ties).toHaveTextContent("中等");
    expect(ties).toHaveTextContent("发出 3");
    expect(ties).toHaveTextContent("一对一说过话");
    expect(ties.textContent ?? "").not.toMatch(/往来较少|往来较多/);
    expect(screen.getByTestId("tie-evidence")).toHaveTextContent("依据 2 条证据");

    // 这一行既要说清节点和边留在本机，也要说清同一页上的「看这个人的摘要」是一条出网路径：
    // 填了端点之后那一按就把这些计数 POST 出去，中间没有确认屏。
    const localOnly = screen.getByTestId("local-only");
    expect(localOnly).toHaveTextContent("只留在本机");
    expect(localOnly).toHaveTextContent("研究预览");
    expect(localOnly).toHaveTextContent("摘要");
    expect(localOnly).toHaveTextContent(/往来次数|发给/);
    // 发出去的不止「往来次数」：`soul-draft` 的 summary_body 把这条边上算出来的
    // 整组陈述都带上——天数、会话数、双方各发多少、有没有一对一、最近一次的日期，
    // 还有本机给的档位。只写「往来次数」等于少说了五件事。
    expect(localOnly).toHaveTextContent("有往来的天数与会话数");
    expect(localOnly).toHaveTextContent("你和对方各发出多少条");
    expect(localOnly).toHaveTextContent("有没有一对一说过话");
    expect(localOnly).toHaveTextContent("最近一次往来的日期");
    expect(localOnly).toHaveTextContent("关系档位");
    expect(localOnly).not.toHaveTextContent("会把这一页上的往来次数发给那个地址");
    expect(localOnly).not.toHaveTextContent("都不进任何出网请求");
    expect(localOnly).not.toHaveTextContent(
      "别人的数据只留在本机：这些节点和边都不进任何出网请求，也不进研究预览。",
    );
  });

  /**
   * The band under a tie is a working hypothesis, and a working hypothesis the
   * user cannot overrule is the black box constraint 10 rules out. Pressing one
   * of the three band words is the whole gesture: the core writes the
   * correction and answers with the graph, and the screen renders what came
   * back rather than patching the row it just pressed.
   *
   * What has to stay on screen afterwards is the machine's own reading. A lock
   * that hid what it overruled would leave the user unable to tell what the
   * counts say about the edge they pinned — which is the same page's promise
   * read backwards.
   */
  it("按下一个档位就是一次纠正：档位改了、锁上了，机器那一读还在屏幕上", async () => {
    const core = await open({ graph: aPeopleGraph() });
    const user = userEvent.setup();
    const tie = () => screen.getByTestId(`tie-${TIE_ID}`);

    expect(within(tie()).queryByTestId(`tie-locked-${TIE_ID}`)).toBeNull();
    await user.click(within(tie()).getByRole("button", { name: "强" }));

    expect(core.callsTo("correct_tie")[0]?.payload).toEqual({
      relationshipId: TIE_ID,
      band: "strong",
    });
    expect(await screen.findByTestId(`tie-locked-${TIE_ID}`)).toHaveTextContent(
      "你改过这一档，重算不再动它",
    );
    expect(tie()).toHaveTextContent("强：往来 6 次");
    // The counts are what a correction does not touch: what was overruled is
    // the one word derived from them.
    expect(tie()).toHaveTextContent("发出 3");
    expect(screen.getByTestId(`tie-machine-${TIE_ID}`)).toHaveTextContent(
      "机器按这些计数算的是「中等」",
    );
    // The row that changed the band is evidence like any other, and the core
    // hands it back resolved with the rest.
    expect(screen.getByTestId("tie-evidence")).toHaveTextContent("依据 3 条证据");
    expect(screen.getByTestId("tie-evidence")).toHaveTextContent("user_correction");
  });

  /**
   * The way back out. A lock with no visible release is a lock whose support
   * burden lands on the user, and the core has `release_tie` for exactly this;
   * what this checks is that the screen offers it only where there is a lock,
   * and that pressing it puts the counts back in charge.
   */
  it("锁住之后才有「按计数重新算」，按下去档位交回给计数", async () => {
    const core = await open({ graph: aPeopleGraph() });
    const user = userEvent.setup();
    const tie = () => screen.getByTestId(`tie-${TIE_ID}`);

    expect(within(tie()).queryByRole("button", { name: "按计数重新算" })).toBeNull();
    await user.click(within(tie()).getByRole("button", { name: "弱" }));
    await screen.findByTestId(`tie-locked-${TIE_ID}`);

    await user.click(within(tie()).getByRole("button", { name: "按计数重新算" }));

    expect(core.callsTo("release_tie")[0]?.payload).toEqual({ relationshipId: TIE_ID });
    await waitFor(() => expect(screen.queryByTestId(`tie-locked-${TIE_ID}`)).toBeNull());
    expect(screen.queryByTestId(`tie-machine-${TIE_ID}`)).toBeNull();
    expect(tie()).toHaveTextContent("中等：往来 6 次");
    expect(within(tie()).queryByRole("button", { name: "按计数重新算" })).toBeNull();
  });

  /**
   * Three words and no fourth: COPY_ZH allows 弱 / 中等 / 强 for a 档位 and the
   * core's `band_named` is a closed set of the same three. The button for the
   * band already in force is grey, the way the profile page greys the position
   * an axis already holds — pressing it would write a correction that changes
   * nothing.
   */
  it("只有那三个档位词，当前这一档的按钮是灰的", async () => {
    await open({ graph: aPeopleGraph() });
    const tie = screen.getByTestId(`tie-${TIE_ID}`);

    const bands = within(tie)
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(bands).toEqual(["弱", "中等", "强"]);
    expect(within(tie).getByRole("button", { name: "中等" })).toBeDisabled();
    expect(within(tie).getByRole("button", { name: "强" })).toBeEnabled();
    expect(screen.getByTestId("ties-explanation")).toHaveTextContent("工作假设");
  });

  /**
   * A refused correction is a value with a reason code on it, and the graph
   * the user was reading stays where it was. The core refuses a word that is
   * not one of the three before it reaches the store, which is the case this
   * fixture stands in for.
   */
  it("核心拒绝这次纠正的时候，屏幕给出理由码，图还是原来那张", async () => {
    await open({
      graph: aPeopleGraph(),
      correctingTie: () => {
        throw { reason_code: "ROUTINE", explanation: NOT_A_BAND_NOTICE };
      },
    });
    const user = userEvent.setup();

    await user.click(
      within(screen.getByTestId(`tie-${TIE_ID}`)).getByRole("button", { name: "强" }),
    );

    expect(await screen.findByTestId("tie-refusal-code")).toHaveTextContent("ROUTINE");
    expect(screen.getByRole("alert")).toHaveTextContent(NOT_A_BAND_NOTICE);
    expect(screen.getByTestId(`tie-${TIE_ID}`)).toHaveTextContent("中等：往来 6 次");
    expect(screen.queryByTestId(`tie-locked-${TIE_ID}`)).toBeNull();
  });

  /**
   * A node's label is sealed text in the store and this screen never opens the
   * seal, so the only way to tell two people apart is the identifier digest.
   * The fixture has a name nowhere in it; this asserts the screen did not go
   * looking for one either, and that you are not listed among the people you
   * talk to.
   */
  it("屏幕上没有人的名字，只有标识摘要，而且你自己不在名单里", async () => {
    await open({ graph: aPeopleGraph() });

    const people = screen.getByTestId("people-list");
    expect(people.textContent ?? "").not.toMatch(/[\u4e00-\u9fa5]{2,3}(先生|女士|同学)/);
    expect(within(people).getByText("bbbbbbbb")).toBeVisible();
    expect(within(people).queryByText("aaaaaaaa")).toBeNull();
    expect(screen.getByRole("heading", { name: "这台机器上看得见的人（1）" })).toBeVisible();
  });

  it("摘要是核心写的那几句，带证据条数和非临床声明", async () => {
    const core = await open({ graph: aPeopleGraph() });
    const user = userEvent.setup();

    await user.click(screen.getAllByRole("button", { name: "看这个人的摘要" })[0]!);

    expect(await screen.findByTestId("summary-text")).toHaveTextContent("一共 6 次往来");
    expect(screen.getByTestId("summary-source")).toHaveTextContent("本机根据往来次数写的统计");
    expect(screen.getByTestId("summary-points")).toHaveTextContent("依据 2 条证据");
    expect(screen.getByTestId("summary-notice")).toHaveTextContent(WORKING_HYPOTHESIS_NOTICE);
    expect(core.callsTo("person_summary")[0]?.payload).toEqual({
      contactId: "0192f000-0000-7000-8000-000000000002",
    });
  });

  /**
   * AC-16's other half: when a line came from the endpoint, the screen has to
   * say so. Draft already renders `source_notice` for the same reason — a
   * degradation the user cannot see is not a degradation, it is the only path
   * there is.
   *
   * What this screen may *not* say is the sentence it used to: 你自己的端点根据
   * 本机统计改写的. The core sends the counts and asks for a rewrite, and it
   * refuses an answer that invents a figure or is about something else — but
   * neither of those makes the sentence a rewrite, and a line that claimed it
   * was would be attributing an endpoint's words to this machine's evidence.
   */
  it("端点写的那一句，屏幕上写明是端点写的，而且不说它是本机统计改写出来的", async () => {
    await open({
      graph: aPeopleGraph(),
      summarizing: () =>
        aPersonSummary({
          source: "user_endpoint",
          text: "你们最近往来比较稳定，多数时候是一对一说话。",
        }),
    });
    const user = userEvent.setup();
    await user.click(screen.getAllByRole("button", { name: "看这个人的摘要" })[0]!);

    const source = await screen.findByTestId("summary-source");
    expect(source).toHaveTextContent("是你自己的端点写的");
    expect(source).toHaveTextContent("没有替你核对");
    expect(source).not.toHaveTextContent("根据本机统计改写");
    expect(screen.getByTestId("summary-text")).toHaveTextContent("往来比较稳定");
    // The evidence-carrying half is still labelled as the counts it is.
    expect(source).toHaveTextContent("本机根据往来次数算的");
    expect(screen.getByTestId("summary-points")).toHaveTextContent("依据 2 条证据");
  });

  /**
   * The other repro from the same probe, as far as this side can carry it: an
   * endpoint answering with 这个人最喜欢榴莲 leaves `soul-draft` handing over
   * the counts, so what has to hold here is that the screen renders a
   * `counts` summary as counts and invents no provenance of its own. The
   * dropping itself is `soul-draft`'s
   * `an_answer_about_something_else_is_dropped_and_the_counts_remain` and
   * `soulcore`'s `session_summary.rs`, where a real endpoint answers a real
   * request.
   */
  it("端点跑题时核心退回计数，屏幕就照计数说，不提端点", async () => {
    await open({
      graph: aPeopleGraph(),
      summarizing: () => aPersonSummary({ source: "counts" }),
    });
    const user = userEvent.setup();
    await user.click(screen.getAllByRole("button", { name: "看这个人的摘要" })[0]!);

    const source = await screen.findByTestId("summary-source");
    expect(source).toHaveTextContent("本机根据往来次数写的统计");
    expect(source).not.toHaveTextContent("端点");
    expect(screen.getByTestId("summary-text")).not.toHaveTextContent("榴莲");
  });

  /**
   * The outbound disclosure on this page names what the request body carries.
   * One of the things it carries is the instruction, and 一句固定的改写要求 was
   * describing something that was not in it: the summary request sent the
   * drafting instruction until `E1Purpose` existed. The sentence now names
   * what `soul_policy::e1::PERSON_SUMMARY_INSTRUCTION` asks for.
   */
  it("出网那一行说的固定指令，就是摘要请求里那一条", async () => {
    await open({ graph: aPeopleGraph() });

    const localOnly = screen.getByTestId("local-only");
    expect(localOnly).toHaveTextContent("固定的系统指令");
    expect(localOnly).toHaveTextContent("不许添新事实");
    expect(localOnly).toHaveTextContent("不许改数字");
  });

  /**
   * The rendered summary goes through the same denylist `xtask` runs over the
   * crates. A fixture that reads like a diagnosis has to fail here, because
   * this is the surface a user reads it on.
   */
  it("渲染出来的摘要里没有一个诊断词或量表词", async () => {
    const terms = diagnosticTerms();
    expect(terms.length).toBeGreaterThan(50);

    await open({
      graph: aPeopleGraph(),
      summarizing: () => aPersonSummary(),
    });
    const user = userEvent.setup();
    await user.click(screen.getAllByRole("button", { name: "看这个人的摘要" })[0]!);
    await screen.findByTestId("summary-text");

    expect(denylistHits(renderedText(), terms)).toEqual([]);
  });

  /**
   * A store that did not open is not a graph with nobody in it, and the screen
   * must not let the two look the same. The refusal arrives from the core as a
   * value with a reason code on it, which is what gets shown.
   */
  it("读不到库的时候给出理由码，而不是一张空的图", async () => {
    installFakeCore({
      graphing: () => {
        throw { reason_code: "STORE_UNAVAILABLE", explanation: "数据库这次没有打开，所以读不到人脉图。" };
      },
    });
    render(<Graph />);

    expect(await screen.findByTestId("graph-refusal-code")).toHaveTextContent("STORE_UNAVAILABLE");
    expect(screen.getByRole("alert")).toHaveTextContent("数据库这次没有打开");
    expect(screen.queryByTestId("no-people")).toBeNull();
  });

  it("整个页面没有碰过网络", async () => {
    const attempts = forbidNetwork();
    await open({ graph: aPeopleGraph() });
    const user = userEvent.setup();

    await user.click(screen.getAllByRole("button", { name: "看这个人的摘要" })[0]!);
    await screen.findByTestId("summary-text");
    await user.click(
      within(screen.getByTestId(`tie-${TIE_ID}`)).getByRole("button", { name: "强" }),
    );
    await screen.findByTestId(`tie-locked-${TIE_ID}`);

    expect(attempts).toEqual([]);
  });
});
