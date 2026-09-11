/**
 * 灵魂档案: the axes, the correction lock, and the words the user wrote that
 * this screen is not allowed to show.
 *
 * `soul-profile` already tests what a correction does to the store. What only
 * this side can check is that the screen renders the core's readings rather
 * than composing its own, that a locked axis still shows the inference it
 * refused, and that a boundary the user typed reaches the page as a question
 * and an id and never as prose.
 *
 * The second half of the file is 再答几题. The wizard runs once and never
 * comes back, so this page is the only place a user who skipped it can state a
 * boundary or a value at all — and the questions it asks have to be the
 * core's, in the core's words, handed in with the core's tokens.
 */

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { Profile } from "./Profile";
import { denylistHits, diagnosticTerms, renderedText } from "../test/denylist";
import {
  anIntakeReceipt,
  aProfileScreen,
  forbidNetwork,
  installFakeCore,
  QUESTIONS,
  WORKING_HYPOTHESIS_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";

const AXIS_ID = "0192b0c0-5001-7a01-8b01-000000000001";
const ORDERLINESS_AXIS_ID = "0192b0c0-5001-7a02-8b02-000000000002";

/** COPY_ZH §7's sentence for `axis_locked_by_user`, as the page renders it. */
const LOCKED_LINE =
  "你纠正过的轴还锁着：这几条已经记进证据里，但没有改动那几条轴的方向。想改方向，就在上面那条轴上直接按你要的那一端。";

/** The same section's fallback, for a reason this build has no line for. */
const OTHER_LINE = "这几条已经记进证据里，但档案没有跟着动。";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Profile />);
  await screen.findByRole("heading", { name: "现在的档案" });
  return core;
}

/** Press 打开题目 and hand back the list the profile page then draws. */
async function askAgain(user: ReturnType<typeof userEvent.setup>): Promise<HTMLElement> {
  const opener = await screen.findByRole("button", { name: "打开题目" });
  await waitFor(() => expect(opener).toBeEnabled());
  await user.click(opener);
  return screen.findByTestId("profile-questions");
}

/** The three buttons of one choice question, found by the question it asks. */
function choicesFor(questionId: string): HTMLElement {
  const prompt = QUESTIONS.find((question) => question.question_id === questionId)?.prompt ?? "";
  return screen.getByRole("group", { name: prompt });
}

/** The text box of one prose question, found the same way. */
function boxFor(questionId: string): HTMLElement {
  const prompt = QUESTIONS.find((question) => question.question_id === questionId)?.prompt ?? "";
  return screen.getByRole("textbox", { name: prompt });
}

/** 条理与执行 as the user left it: pinned themselves, and locked (AC-07). */
function withCorrectedOrderliness() {
  return aProfileScreen({
    axes: aProfileScreen().axes.map((axis) =>
      axis.axis_id === ORDERLINESS_AXIS_ID
        ? {
            ...axis,
            position: "leans_low",
            reading: "条理与执行：偏向随性推进",
            locked_by_user: true,
          }
        : axis,
    ),
  });
}

/** The frozen copy file, read the way `projection_sentences.rs` reads it. */
function frozenCopy(): string {
  const here = dirname(fileURLToPath(import.meta.url));
  return readFileSync(join(here, "..", "..", "..", "..", "docs", "algorithms", "COPY_ZH.md"), "utf8");
}

function answersSentBy(core: Awaited<ReturnType<typeof open>>) {
  return (
    core.callsTo("answer_questionnaire")[0]?.payload as {
      answers: { question_id: string; given: string }[];
    }
  ).answers;
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

  /**
   * Where a direction came from is a claim about this build, not about the
   * shape the store could hold. `record_axis_inference` exists in
   * `soul-profile`, but nothing on this side calls it and committing an import
   * rebuilds the people graph rather than proposing an axis — so the only
   * things that have ever moved an axis here are the questionnaire and the
   * buttons on this page, and the sentence must not borrow /graph's 导入的往来.
   */
  it("轴的说明写的是题目和纠正，不说方向是从导入的往来里推出来的", async () => {
    await open();

    const explanation = screen.getByTestId("axes-explanation");
    expect(explanation).toHaveTextContent("你答过");
    expect(explanation).toHaveTextContent("纠正");
    expect(explanation.textContent ?? "").not.toMatch(/导入/);
    expect(explanation).toHaveTextContent("只有偏向，没有高低，也没有名次");
    // The lock is real in the core, and it is a lock against inference rather
    // than against the user answering the same question again.
    expect(explanation).toHaveTextContent("锁住");
    expect(explanation).toHaveTextContent("不会覆盖你");
    // Pinning the end an axis already leans is a gesture the page has to name,
    // or it is a lock only somebody who read the source would find.
    expect(explanation).toHaveTextContent("按当前那一端把它锁住");
  });

  /**
   * Agreeing with an axis is a correction too. `soul_profile::correct_axis`
   * locks whichever position it is handed and the intake path locks nothing,
   * so pressing the end an axis already leans is the only way to say
   * 这一端就对了，别再推它 — and the page used to grey exactly that button, on
   * the rationale that pressing it would change nothing. It changes the lock.
   * With it grey the way round was to pin the other end and come back, which
   * leaves a `UserCorrection` in the store asserting a position the user never
   * held.
   */
  it("按下轴现在这一端也是一次纠正：没锁的时候按得下去，按完就锁住了", async () => {
    const core = await open();
    const user = userEvent.setup();
    const row = () => screen.getByTestId(`axis-${AXIS_ID}`);

    expect(within(row()).queryByTestId(`axis-locked-${AXIS_ID}`)).toBeNull();
    const held = within(row()).getByRole("button", { name: "偏向尝试新的做法" });
    expect(held).toBeEnabled();
    await user.click(held);

    expect(core.callsTo("correct_axis")[0]?.payload).toEqual({
      axisId: AXIS_ID,
      position: "leans_high",
    });
    expect(await screen.findByTestId(`axis-locked-${AXIS_ID}`)).toHaveTextContent(
      "推断不再改这条",
    );
    expect(screen.getByTestId(`axis-reading-${AXIS_ID}`)).toHaveTextContent(
      "偏向尝试新的做法",
    );

    // Now the old rationale is the true one: the verdict is recorded, so
    // restating it is the one press that would change nothing.
    expect(within(row()).getByRole("button", { name: "偏向尝试新的做法" })).toBeDisabled();
    expect(within(row()).getByRole("button", { name: "偏向熟悉稳妥的做法" })).toBeEnabled();
    expect(within(row()).getByRole("button", { name: "两端都有，看场合" })).toBeEnabled();
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
    // With the eleven questions open too: they are prose on this page as much
    // as the axis readings are, and nobody else checks them after a render.
    await askAgain(userEvent.setup());

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
    await askAgain(userEvent.setup());

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/发送|发出|执行|上传|导出/);
    }
    expect(attempts).toEqual([]);
  });
});

describe("在档案页上再答几题", () => {
  /**
   * The wizard tells a user who answered nothing that the questions are still
   * on this page. That sentence is only true if the section it names is here,
   * so the promise is read off the wizard's own source and then looked for on
   * this screen.
   */
  it("向导许诺的那一节，在这一页上确实有", async () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const wizard = readFileSync(join(here, "Wizard.tsx"), "utf8");
    const promised = /档案页的「(.+?)」里随时可以再答/.exec(wizard)?.[1] ?? "";
    expect(promised).toBe("再答几题");

    await open();

    expect(screen.getByRole("heading", { name: promised })).toBeVisible();
  });

  /**
   * The same eleven, from the same command. Not a shorter list somebody
   * assembled for this page: the ids are the ones `questionnaire()` sent, the
   * three prose questions among them, because those three are the only way
   * anything ever reaches 边界 or 在乎的事.
   */
  it("这一页给的题就是核心那一份，三道填空题一道不少", async () => {
    const core = await open();
    const questions = await askAgain(userEvent.setup());

    expect(core.callsTo("questionnaire")).toHaveLength(1);
    expect(within(questions).getAllByRole("listitem")).toHaveLength(QUESTIONS.length);
    for (const question of QUESTIONS) {
      expect(document.getElementById(`profile-question-${question.question_id}`)).toHaveTextContent(
        question.prompt,
      );
    }
    for (const id of ["q.boundary.topics", "q.boundary.availability", "q.value.what_matters"]) {
      expect(boxFor(id)).toBeVisible();
    }
    expect(choicesFor("q.axis.curiosity")).toBeVisible();
    expect(choicesFor("q.axis.orderliness")).toBeVisible();
    expect(within(questions).getAllByRole("textbox")).toHaveLength(3);
  });

  /**
   * The hole this section fills: an axis the wizard skipped stays 还看不出方向
   * forever unless something can answer for it later. What goes back is the
   * core's own token, and what comes back is the whole screen — the axis is
   * re-read rather than patched here.
   */
  it("补答一条跳过的轴，交上去之后这条轴有了方向", async () => {
    const core = await open();
    const user = userEvent.setup();
    await askAgain(user);

    expect(within(screen.getByTestId("axis-list")).getByText("条理与执行：还看不出方向")).toBeVisible();

    await user.click(within(choicesFor("q.axis.orderliness")).getByRole("button", { name: "偏那一端" }));
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    expect(core.callsTo("answer_questionnaire")).toHaveLength(1);
    const answers = answersSentBy(core);
    expect(answers).toHaveLength(QUESTIONS.length);
    expect(answers.filter((answer) => answer.given !== "")).toEqual([
      { question_id: "q.axis.orderliness", given: "leans_high" },
    ]);

    expect(
      await within(screen.getByTestId(`axis-${ORDERLINESS_AXIS_ID}`)).findByText(
        "条理与执行：偏向先规划再动手",
      ),
    ).toBeVisible();
    expect(core.callsTo("profile_screen")).toHaveLength(2);
    expect(screen.getByTestId("profile-known")).toHaveTextContent("五条轴里有 2 条有方向");
    expect(screen.getByTestId("profile-receipt")).toHaveTextContent("记下了 1 条");
  });

  it("锁定轴被跳过时，回执列出编号和原因", async () => {
    const core = await open({
      recording: () =>
        anIntakeReceipt({
          answered: 0,
          ignored: [
            { question_id: "q.axis.orderliness", reason: "axis_locked_by_user" },
          ],
        }),
    });
    const user = userEvent.setup();
    await askAgain(user);
    await user.click(
      within(choicesFor("q.axis.orderliness")).getByRole("button", { name: "偏那一端" }),
    );
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    const ignored = await screen.findByTestId("profile-ignored");
    expect(ignored).toHaveTextContent("q.axis.orderliness");
    expect(ignored).toHaveTextContent("axis_locked_by_user");
    expect(core.callsTo("answer_questionnaire")).toHaveLength(1);
  });

  /**
   * AC-03's other half, reached from the product path rather than the wizard:
   * a user who skipped the three fill-ins can state a boundary here, and what
   * they typed goes to the core and comes back as a question and an id. The
   * words themselves are looked for across the whole rendered document, which
   * is the one place a leak would show.
   */
  it("在这里写下的边界只交给核心，原话不回到屏幕上", async () => {
    const written = "周末和家里人的事一概不要碰";
    const core = await open({ profile: () => aProfileScreen({ stated: [] }) });
    const user = userEvent.setup();

    expect(screen.getByTestId("no-stated")).toHaveTextContent("再答几题");
    await askAgain(user);

    await user.type(boxFor("q.boundary.topics"), written);
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    expect(answersSentBy(core).filter((answer) => answer.given !== "")).toEqual([
      { question_id: "q.boundary.topics", given: written },
    ]);
    const stated = await screen.findByTestId("stated-list");
    expect(stated).toHaveTextContent("边界");
    expect(stated).toHaveTextContent("有哪些话题");
    expect(stated).toHaveTextContent("原话密封");
    expect(renderedText()).not.toContain(written);
    // The box is not a second copy of the answer either: it goes back to
    // blank, because what the core now holds is on the screen above it.
    expect(boxFor("q.boundary.topics")).toHaveValue("");
  });

  /**
   * A questionnaire with nothing in it is refused by the core, so the button
   * stays grey rather than sending one and reporting the refusal afterwards.
   * Nothing is claimed either way: no receipt, and the profile is not re-read.
   */
  it("一道都没答的时候交不上去，也不会假装记下了什么", async () => {
    const core = await open();
    const user = userEvent.setup();
    await askAgain(user);

    expect(screen.getByTestId("profile-answered")).toHaveTextContent("这一次答了 0 题");
    const hand = screen.getByRole("button", { name: "写进档案" });
    expect(hand).toBeDisabled();
    await user.click(hand);

    expect(core.callsTo("answer_questionnaire")).toHaveLength(0);
    expect(core.callsTo("profile_screen")).toHaveLength(1);
    expect(screen.queryByTestId("profile-receipt")).toBeNull();
    expect(screen.getByTestId("profile-nothing-answered")).toHaveTextContent("没有东西可以写进去");
  });

  /** An answer withdrawn is a blank again, and a blank is a skip. */
  it("再按一下就是收回，收回之后又交不上去了", async () => {
    const core = await open();
    const user = userEvent.setup();
    await askAgain(user);

    const choice = within(choicesFor("q.axis.curiosity")).getByRole("button", { name: "偏这一端" });
    await user.click(choice);
    expect(screen.getByTestId("profile-answered")).toHaveTextContent("这一次答了 1 题");
    await user.click(choice);
    expect(screen.getByTestId("profile-answered")).toHaveTextContent("这一次答了 0 题");
    expect(screen.getByRole("button", { name: "写进档案" })).toBeDisabled();
    expect(core.callsTo("answer_questionnaire")).toHaveLength(0);
  });

  /** A refusal is the core's own sentence, and the profile is left alone. */
  it("核心拒绝的时候照抄核心的话，档案不动", async () => {
    const core = await open({
      recording: () => {
        throw { reason_code: "ROUTINE", explanation: "这份问卷核心不收。" };
      },
    });
    const user = userEvent.setup();
    await askAgain(user);

    await user.click(within(choicesFor("q.axis.curiosity")).getByRole("button", { name: "偏这一端" }));
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    expect(await screen.findByTestId("profile-ask-refusal-code")).toHaveTextContent("ROUTINE");
    expect(screen.getByText("这份问卷核心不收。")).toBeVisible();
    expect(core.callsTo("profile_screen")).toHaveLength(1);
    expect(screen.queryByTestId("profile-receipt")).toBeNull();
  });

  /**
   * 以你最后说的为准 was true of every question until AC-07's lock arrived. It
   * is true now of the axes the user has not corrected and false of the ones
   * they have (D46), so the promise has to carry the qualifier wherever it is
   * made — a user who reads it, re-answers a corrected axis and watches
   * nothing move was told the wrong thing by this page.
   */
  it("最后说的为准只管没锁的轴，锁上的轴写明再答只记证据", async () => {
    await open();
    const said = (screen.getByTestId("ask-again-explanation").textContent ?? "").replace(
      /\s+/g,
      "",
    );

    expect(said).toContain("没锁住的那些答过也可以改口，以你最后说的为准");
    expect(said).toContain("纠正过的轴已经锁住了，再答一次不会把它改回去");
    expect(said).toContain("照样记进证据里");
    expect(said).toContain("不动那条轴的方向");
    // Never the bare promise: every 以你最后说的为准 on this page is the tail
    // of a clause that has already said which axes it is about.
    for (const found of said.matchAll(/以你最后说的为准/g)) {
      expect(said.slice(0, found.index)).toMatch(/没锁[^。]*$/);
    }
  });

  /**
   * D46 reached from the product path: an answer that lands on an axis the
   * user corrected is written and not applied, and the receipt says so. A run
   * that reported only 记下了 N 条 would be describing an intake that did more
   * than it did — and the axis on screen would be the only clue.
   */
  it("答在锁住的轴上，这一条没有生效，收据说得出来是哪几条", async () => {
    const core = await open({ profile: withCorrectedOrderliness });
    const user = userEvent.setup();
    await askAgain(user);

    await user.click(
      within(choicesFor("q.axis.curiosity")).getByRole("button", { name: "偏这一端" }),
    );
    await user.click(
      within(choicesFor("q.axis.orderliness")).getByRole("button", { name: "偏那一端" }),
    );
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    const ignored = await screen.findByTestId("profile-ignored");
    expect(ignored).toHaveTextContent("这一次有 1 条没有改动档案");
    expect(ignored).toHaveTextContent(LOCKED_LINE);
    // The other answer did land, and the receipt counts that one alone.
    expect(screen.getByTestId("profile-receipt")).toHaveTextContent("记下了 1 条");
    expect(core.callsTo("answer_questionnaire")).toHaveLength(1);
    expect(
      within(screen.getByTestId(`axis-${ORDERLINESS_AXIS_ID}`)).getByText(
        "条理与执行：偏向随性推进",
      ),
    ).toBeVisible();
    expect(screen.getByTestId(`axis-locked-${ORDERLINESS_AXIS_ID}`)).toBeVisible();
  });

  /** Nothing was refused, so nothing is said about refusals. */
  it("没有被拒的答案时，这一段一个字都不出现", async () => {
    await open();
    const user = userEvent.setup();
    await askAgain(user);

    await user.click(
      within(choicesFor("q.axis.orderliness")).getByRole("button", { name: "偏那一端" }),
    );
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    expect(await screen.findByTestId("profile-receipt")).toHaveTextContent("记下了 1 条");
    expect(screen.queryByTestId("profile-ignored")).toBeNull();
    expect(renderedText()).not.toContain("没有改动档案");
  });

  /**
   * `axis_locked_by_user` is a word for a machine to match on. The page looks
   * the token up and prints the sentence COPY_ZH froze for it; two answers
   * stopped by the same lock are one sentence, because the count beside it is
   * what says how many there were. A reason this build has never heard of
   * prints what is true of any refused answer — and still not the token.
   */
  it("屏幕上出现的是话不是机器词，同一个理由只说一次", async () => {
    await open({
      recording: () =>
        anIntakeReceipt({
          answered: 1,
          ignored: [
            { question_id: "q.axis.curiosity", reason: "axis_locked_by_user" },
            { question_id: "q.axis.orderliness", reason: "axis_locked_by_user" },
            { question_id: "q.axis.social_energy", reason: "a_reason_this_build_never_heard_of" },
          ],
        }),
    });
    const user = userEvent.setup();
    await askAgain(user);

    await user.click(
      within(choicesFor("q.axis.curiosity")).getByRole("button", { name: "偏这一端" }),
    );
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    const ignored = await screen.findByTestId("profile-ignored");
    expect(ignored).toHaveTextContent("这一次有 3 条没有改动档案");
    expect((ignored.textContent ?? "").split(LOCKED_LINE)).toHaveLength(2);
    expect(ignored).toHaveTextContent(OTHER_LINE);
    for (const token of ["axis_locked_by_user", "a_reason_this_build_never_heard_of"]) {
      expect(renderedText()).not.toContain(token);
    }
  });

  /**
   * Copy reaches COPY_ZH before it reaches a screen, the way
   * `projection_sentences.rs` pins the demotion clock's four templates: the
   * key is the token the core sends, and the line under it is word for word
   * what this page renders.
   */
  it("这几句话逐字来自冻结的话术文件", async () => {
    const copy = frozenCopy();
    const lineFor = (key: string): string =>
      copy.split("\n").find((line) => line.includes(key)) ?? "";

    expect(lineFor("profile.intake.ignored_count")).toContain("这一次有 {条数} 条没有改动档案。");
    expect(lineFor("profile.intake.axis_locked_by_user")).toContain(LOCKED_LINE);
    expect(lineFor("profile.intake.ignored_other")).toContain(OTHER_LINE);

    await open({ profile: withCorrectedOrderliness });
    const user = userEvent.setup();
    await askAgain(user);
    await user.click(
      within(choicesFor("q.axis.orderliness")).getByRole("button", { name: "偏那一端" }),
    );
    await user.click(screen.getByRole("button", { name: "写进档案" }));

    expect((await screen.findByTestId("profile-ignored")).textContent).toBe(
      `这一次有 1 条没有改动档案。${LOCKED_LINE}`,
    );
  });
});
