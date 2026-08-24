/**
 * 灵魂档案: the axes, the correction lock, and the words the user wrote that
 * this screen is not allowed to show.
 *
 * `soul-profile` already tests what a correction does to the store. What only
 * this side can check is that the screen renders the core's readings rather
 * than composing its own, that a locked axis still shows the inference it
 * refused, and that a boundary the user typed reaches the page as a question
 * and an id and never as prose.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { Profile } from "./Profile";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  aProfileScreen,
  forbidNetwork,
  installFakeCore,
  WORKING_HYPOTHESIS_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";

const AXIS_ID = "0192b0c0-5001-7a01-8b01-000000000001";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Profile />);
  await screen.findByRole("heading", { name: "现在的档案" });
  return core;
}

describe("灵魂档案页", () => {
  it("整段读法是核心写的，界面不自己拼句子", async () => {
    await open();

    expect(screen.getByTestId("profile-reading")).toHaveTextContent("偏向尝试新的做法");
    expect(screen.getByTestId("profile-notice")).toHaveTextContent(WORKING_HYPOTHESIS_NOTICE);
    expect(screen.getByTestId("voice-reading")).toHaveTextContent("平实");
  });

  /**
   * 不猜 is the whole of the partial-questionnaire promise, and the screen has
   * to say it out loud rather than leaving four blank rows.
   */
  it("没答过的轴写着还看不出方向，并且说明不会替你猜", async () => {
    await open();

    expect(screen.getByTestId("profile-known")).toHaveTextContent("五条轴里有 1 条有方向");
    expect(screen.getByTestId("profile-known")).toHaveTextContent("没答过的题不会被猜");
    const axes = screen.getByTestId("axis-list");
    expect(within(axes).getByText("条理与执行：还看不出方向")).toBeVisible();
  });

  /** AC-07: the correction goes to the core, and the whole screen comes back. */
  it("纠正一条轴之后，这条轴锁住了", async () => {
    const core = await open();
    const user = userEvent.setup();

    const row = screen.getByTestId(`axis-${AXIS_ID}`);
    await user.click(within(row).getByRole("button", { name: "偏向熟悉稳妥的做法" }));

    expect(core.callsTo("correct_axis")[0]?.payload).toEqual({
      axisId: AXIS_ID,
      position: "leans_low",
    });
    expect(await screen.findByTestId(`axis-locked-${AXIS_ID}`)).toHaveTextContent(
      "推断不再改这条",
    );
  });

  /**
   * A lock that hid the inference it refused would make AC-07 invisible. The
   * user has to be able to see that the machine still disagrees and is not
   * allowed to act on it.
   */
  it("锁住之后，机器那条不同意的推断还留在屏幕上", async () => {
    const second = "0192b0c0-5001-7a02-8b02-000000000002";
    await open({
      profile: () =>
        aProfileScreen({
          axes: aProfileScreen().axes.map((axis) =>
            axis.axis_id === second ? { ...axis, locked_by_user: true } : axis,
          ),
        }),
    });

    const inferences = screen.getByTestId(`axis-inferences-${second}`);
    expect(inferences).toHaveTextContent("机器推的是「偏向随性推进」");
    expect(inferences).toHaveTextContent("因为你锁定过所以没有生效");
  });

  it("设一项语气之后，这一项也锁住了", async () => {
    const core = await open();
    const user = userEvent.setup();

    const row = screen.getByTestId("voice-register");
    await user.click(within(row).getByRole("button", { name: "正式" }));

    expect(core.callsTo("set_voice")[0]?.payload).toEqual({ field: "register", option: "formal" });
    expect(await screen.findByTestId("voice-locked-register")).toBeVisible();
  });

  /**
   * The user's own words about their boundaries are sealed in the event the
   * recorder wrote. `StatedRow` has no field that could carry them, so what is
   * on screen is the question and the evidence id — and this checks the answer
   * itself is nowhere on the page.
   */
  it("你写过的边界只留问题和编号，原话不出现", async () => {
    await open({
      profile: () =>
        aProfileScreen({
          stated: [
            {
              field: "boundary",
              question_id: "q.boundary.topics",
              prompt: "有哪些话题，你不希望 Soul 替你起草或分析？",
              event_id: "0192f000-0000-7000-8000-0000000000b1",
              evidence_id: "0192f000-0000-7000-8000-0000000000b2",
            },
          ],
        }),
    });

    const stated = screen.getByTestId("stated-list");
    expect(stated).toHaveTextContent("边界");
    expect(stated).toHaveTextContent("有哪些话题");
    expect(stated).toHaveTextContent("0192f000-0000-7000-8000-0000000000b2");
    expect(stated).toHaveTextContent("原话密封");
  });

  it("一条都没写过的时候，不替用户编一条出来", async () => {
    await open({ profile: () => aProfileScreen({ stated: [] }) });

    expect(screen.getByTestId("no-stated")).toHaveTextContent("跳过就是跳过");
    expect(screen.queryByTestId("stated-list")).toBeNull();
  });

  /** The same denylist `xtask` runs over the crates, over what was rendered. */
  it("渲染出来的档案里没有一个诊断词或量表词", async () => {
    const terms = diagnosticTerms();
    expect(terms.length).toBeGreaterThan(50);

    await open({ profile: () => aProfileScreen() });

    expect(denylistHits(renderedText(), terms)).toEqual([]);
  });

  /** A store that did not open is a refusal with a code, not a blank profile. */
  it("读不到库的时候给出理由码，而不是一份空档案", async () => {
    installFakeCore({
      profile: () => {
        throw {
          reason_code: "STORE_UNAVAILABLE",
          explanation: "数据库这次没有打开，所以读不到档案。",
        };
      },
    });
    render(<Profile />);

    expect(await screen.findByTestId("profile-refusal-code")).toHaveTextContent(
      "STORE_UNAVAILABLE",
    );
    expect(screen.queryByTestId("axis-list")).toBeNull();
  });

  it("这一页上没有一个按钮是发送或执行", async () => {
    const attempts = forbidNetwork();
    await open();

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/发送|发出|执行|上传|导出/);
    }
    expect(attempts).toEqual([]);
  });
});
