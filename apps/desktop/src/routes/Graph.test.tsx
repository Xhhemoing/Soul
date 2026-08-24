/**
 * The people screen: counts and evidence, no names, and nothing a medical
 * product would say.
 *
 * `soul-draft` already tests that a summary cites evidence and passes the
 * denylist. What only this side can check is that the screen renders the
 * core's sentences rather than composing its own, and that a graph with
 * somebody in it still has no name on it.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { Graph } from "./Graph";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  aPeopleGraph,
  aPersonSummary,
  forbidNetwork,
  installFakeCore,
  WORKING_HYPOTHESIS_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";

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
    expect(ties).toHaveTextContent("发出 3");
    expect(ties).toHaveTextContent("一对一说过话");
    expect(screen.getByTestId("tie-evidence")).toHaveTextContent("依据 2 条证据");
    expect(screen.getByTestId("local-only")).toHaveTextContent("只留在本机");
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
    expect(screen.getByTestId("summary-points")).toHaveTextContent("依据 2 条证据");
    expect(screen.getByTestId("summary-notice")).toHaveTextContent(WORKING_HYPOTHESIS_NOTICE);
    expect(core.callsTo("person_summary")[0]?.payload).toEqual({
      contactId: "0192f000-0000-7000-8000-000000000002",
    });
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

    expect(attempts).toEqual([]);
  });
});
