/**
 * AC-02, shell side: the first-run wizard finishes with everything off, and it
 * has no way to turn anything on.
 *
 * AC-03, shell side: it also finishes with a profile. The eleven questions are
 * the only path to one for a user who imports nothing, so the tests below care
 * about three things — that the wizard draws the questions the core sent
 * rather than a list of its own, that what it hands back is what the user
 * tapped, and that a question left alone is handed back blank so the axis it
 * would have moved stays 还看不出方向.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ConfigSnapshot } from "../core";
import {
  CLOSED_SNAPSHOT,
  CLOUD_LABEL,
  forbidNetwork,
  installFakeCore,
  QUESTIONS,
  type FakeCoreOptions,
} from "../test/fakeCore";
import { Wizard } from "./Wizard";

function renderWizard(options: FakeCoreOptions = {}) {
  const snapshot = options.snapshot ?? CLOSED_SNAPSHOT;
  const core = installFakeCore({ ...options, snapshot });
  const onComplete = vi.fn();
  render(<Wizard snapshot={snapshot} onComplete={onComplete} />);
  return { core, onComplete };
}

/** Read the defaults, tick the box, and get to the questions. */
async function readTheDefaults(user: ReturnType<typeof userEvent.setup>): Promise<HTMLElement> {
  await user.click(screen.getByRole("checkbox"));
  await user.click(screen.getByRole("button", { name: "下一步" }));
  return screen.findByTestId("wizard-questions");
}

/** Answer one choice question by the words the core sent for the option. */
async function answerFirstAxis(user: ReturnType<typeof userEvent.setup>): Promise<void> {
  const questions = await screen.findByTestId("wizard-questions");
  const first = within(questions).getAllByRole("group")[0] as HTMLElement;
  await user.click(within(first).getAllByRole("button")[2] as HTMLElement);
}

describe("首次向导", () => {
  it("每一项能力在开始使用之前都是关的", async () => {
    const { core, onComplete } = renderWizard();
    const user = userEvent.setup();

    expect(screen.getByTestId("wizard-state-前台应用使用时长采集")).toHaveTextContent("关");
    expect(screen.getByTestId("wizard-state-云端深度分析")).toHaveTextContent(CLOUD_LABEL);
    expect(screen.getByTestId("wizard-state-语言模型端点")).toHaveTextContent("未填写");
    expect(screen.getByTestId("wizard-state-已授权目录")).toHaveTextContent("0 个");

    await readTheDefaults(user);
    await answerFirstAxis(user);
    await user.click(screen.getByRole("button", { name: "写进档案，开始使用" }));

    expect(onComplete).toHaveBeenCalledTimes(1);
    const completed = onComplete.mock.calls[0]?.[0] as ConfigSnapshot;
    expect(completed.collect_enabled).toBe(false);
    expect(completed.cloud.enabled).toBe(false);
    expect(completed.cloud.state).toBe("not_yet_available");
    expect(completed.llm_endpoint_configured).toBe(false);
    expect(completed.authorized_root_count).toBe(0);
    expect(completed.fully_closed).toBe(true);
    expect(completed.open_capabilities).toEqual([]);

    expect(core.callsTo("complete_wizard")).toHaveLength(1);
    expect(core.callsTo("complete_wizard")[0]?.payload).toEqual({
      answers: { acknowledged_defaults_are_off: true },
    });
  });

  /**
   * The 已授权目录 note must not read as "点头之后 Soul 就能写文件了". Both halves
   * have to hold on their own: the scanner reads nothing without authorization,
   * and this version writes nothing into a scanned directory even after it —
   * the same read-only promise `READ_ONLY_NOTICE` makes on 文件整理.
   */
  it("授权目录那一行不会暗示点头之后就能写文件", () => {
    renderWizard();

    const note = screen.getByTestId("wizard-note-已授权目录");
    expect(note).toHaveTextContent("不看任何目录");
    expect(note).toHaveTextContent("只做只读扫描与计划预览");
    expect(note).toHaveTextContent("不会写、移动、重命名或删除被扫的任何文件");
    expect(note.textContent).not.toContain("没有你点头，Soul 不看任何目录，也不会写任何文件。");
  });

  /**
   * Neither half may be said of Soul as a whole. Without a root on this list
   * this build still reads the file the user picks on /import and still reads
   * and writes its own config, keys and database — so an unqualified
   * 「Soul 不看任何目录」/「不会写任何文件」 is false on the screen whose whole
   * job is telling a first-run user what is off. The scanner is what the row
   * counts and the scanner is what the sentence may promise about.
   */
  it("授权目录那一行把读与写都限定在目录扫描器上，并点名两个例外", () => {
    renderWizard();

    const note = screen.getByTestId("wizard-note-已授权目录");
    expect(note).toHaveTextContent("Soul 的目录扫描不看任何目录");
    expect(note).toHaveTextContent("导入页");
    expect(note).toHaveTextContent("自己的配置与数据库");
    expect(note.textContent).not.toContain("没有你点头，Soul 不看任何目录。");
    expect(note.textContent).not.toContain("不会写任何文件");
  });

  /**
   * The welcome line must not promise that every later process stays on this
   * machine. Filling an endpoint on 设置 makes draft generate and person
   * summary leave (redacted). The sentence is true of this screen's moment —
   * nothing is filled yet — but it is the last thing the user reads about
   * locality before they never see this page again.
   */
  it("欢迎那段不会把后来的模型端点说成也留在本机", () => {
    renderWizard();

    const pitch = screen.getByTestId("wizard-locality");
    expect(pitch).toHaveTextContent("留在本机");
    expect(pitch).toHaveTextContent("模型端点");
    expect(pitch).toHaveTextContent("占位");
    expect(pitch.textContent).not.toContain("它只处理你交给它的东西，处理过程留在本机。");
  });

  /**
   * "不弹一堆权限" is checkable: the only tick box on the screen is the one
   * that says the user read the page. A permission checkbox added later fails
   * here before anyone has to notice it in review.
   */
  it("除了「我读过」之外没有第二个勾选框", async () => {
    renderWizard();
    const user = userEvent.setup();
    await readTheDefaults(user);

    const boxes = screen.getAllByRole("checkbox");
    expect(boxes).toHaveLength(1);
    expect(screen.getByLabelText(/我读过上面这几行/)).toBe(boxes[0]);
  });

  it("没有勾选之前不能往下走，也不会去问核心", async () => {
    const { core } = renderWizard();
    const user = userEvent.setup();

    const next = screen.getByRole("button", { name: "下一步" });
    expect(next).toBeDisabled();
    await user.click(next);

    expect(screen.queryByTestId("wizard-questions")).toBeNull();
    expect(core.callsTo("complete_wizard")).toHaveLength(0);
    expect(core.callsTo("answer_questionnaire")).toHaveLength(0);
  });

  it("走完整个向导不碰网络", async () => {
    const attempts = forbidNetwork();
    const { onComplete } = renderWizard();
    const user = userEvent.setup();

    await readTheDefaults(user);
    await answerFirstAxis(user);
    await user.click(screen.getByRole("button", { name: "写进档案，开始使用" }));

    expect(onComplete).toHaveBeenCalledTimes(1);
    expect(attempts).toEqual([]);
  });

  /**
   * If the core ever hands back an open configuration, the wizard must show it
   * rather than describe it as closed. The sentence on that screen is read
   * from the snapshot for exactly this reason.
   */
  it("核心若说某项是开的，向导照实显示", () => {
    renderWizard({
      snapshot: {
        ...CLOSED_SNAPSHOT,
        collect_enabled: true,
        llm_endpoint_configured: true,
        authorized_root_count: 2,
        fully_closed: false,
        open_capabilities: ["collect_enabled", "llm_endpoint"],
      },
    });

    expect(screen.getByTestId("wizard-state-前台应用使用时长采集")).toHaveTextContent("开");
    expect(screen.getByTestId("wizard-state-语言模型端点")).toHaveTextContent("已填写");
    expect(screen.getByTestId("wizard-state-已授权目录")).toHaveTextContent("2 个");
  });
});

describe("向导里的十一道题", () => {
  /**
   * The questions come from the core, not from this shell. What is on screen
   * is the list `soul_import::questionnaire::QUESTIONS` declares, in its
   * order, with the option words `soul-profile` owns.
   */
  it("题目和选项都是核心发过来的那一份", async () => {
    const { core } = renderWizard();
    const user = userEvent.setup();
    const questions = await readTheDefaults(user);

    expect(core.callsTo("questionnaire")).toHaveLength(1);
    expect(within(questions).getAllByRole("listitem")).toHaveLength(QUESTIONS.length);
    for (const question of QUESTIONS) {
      expect(within(questions).getByText(question.prompt)).toBeVisible();
    }
    // Three text boxes, one per prose question, and no other free-text field.
    expect(within(questions).getAllByRole("textbox")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "偶尔用表情" })).toBeVisible();
  });

  /** AC-03 through the shell: the wizard finishes and the profile is not empty. */
  it("答完之后档案里有东西，而且都是「你自己说的」", async () => {
    const { core, onComplete } = renderWizard();
    const user = userEvent.setup();
    await readTheDefaults(user);

    await answerFirstAxis(user);
    await user.click(screen.getByRole("button", { name: "正式" }));
    await user.type(screen.getAllByRole("textbox")[0] as HTMLElement, "工作以外的事");
    await user.click(screen.getByRole("button", { name: "写进档案，开始使用" }));

    const recorded = core.callsTo("answer_questionnaire");
    expect(recorded).toHaveLength(1);
    const answers = (recorded[0]?.payload as { answers: { question_id: string; given: string }[] })
      .answers;
    expect(answers).toHaveLength(QUESTIONS.length);
    expect(answers.filter((answer) => answer.given !== "")).toEqual([
      { question_id: "q.axis.curiosity", given: "leans_high" },
      { question_id: "q.voice.register", given: "formal" },
      { question_id: "q.boundary.topics", given: "工作以外的事" },
    ]);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  /**
   * The other half of AC-03: a question nobody answered is handed back blank,
   * and the core drops it. Nothing on this screen fills one in.
   */
  it("跳过的题原样交回去，不替用户猜", async () => {
    const { core } = renderWizard();
    const user = userEvent.setup();
    await readTheDefaults(user);
    await answerFirstAxis(user);

    expect(screen.getByTestId("wizard-answered")).toHaveTextContent("已答 1 题，跳过 10 题");
    await user.click(screen.getByRole("button", { name: "写进档案，开始使用" }));

    const answers = (
      core.callsTo("answer_questionnaire")[0]?.payload as {
        answers: { question_id: string; given: string }[];
      }
    ).answers;
    expect(answers.filter((answer) => answer.given === "")).toHaveLength(QUESTIONS.length - 1);
  });

  /** An answer can be taken back, and taking it back is a skip. */
  it("再按一下就是收回，收回等于跳过", async () => {
    const { core } = renderWizard();
    const user = userEvent.setup();
    await readTheDefaults(user);

    await answerFirstAxis(user);
    expect(screen.getByTestId("wizard-answered")).toHaveTextContent("已答 1 题");
    await answerFirstAxis(user);
    expect(screen.getByTestId("wizard-answered")).toHaveTextContent("已答 0 题");
    expect(screen.getByRole("button", { name: "写进档案，开始使用" })).toBeDisabled();
    expect(core.callsTo("answer_questionnaire")).toHaveLength(0);
  });

  /**
   * Skipping the whole thing is allowed and does not pretend otherwise: the
   * wizard finishes without recording anything, and the profile stays empty
   * until the user answers on the profile page.
   */
  it("一题都不答也能开始，但不会假装写过档案", async () => {
    const { core, onComplete } = renderWizard();
    const user = userEvent.setup();
    await readTheDefaults(user);

    await user.click(screen.getByRole("button", { name: "一题都不答，直接开始" }));

    expect(core.callsTo("answer_questionnaire")).toHaveLength(0);
    expect(core.callsTo("complete_wizard")).toHaveLength(1);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  /**
   * A refusal keeps the user on this page with the core's own sentence in
   * front of them, rather than dropping them into a shell whose profile is
   * empty for a reason nobody read.
   */
  it("核心拒绝的时候留在这一页，并且照抄核心的话", async () => {
    const { core, onComplete } = renderWizard({
      recording: () => {
        throw { reason_code: "ROUTINE", explanation: "这份问卷核心不收。" };
      },
    });
    const user = userEvent.setup();
    await readTheDefaults(user);
    await answerFirstAxis(user);
    await user.click(screen.getByRole("button", { name: "写进档案，开始使用" }));

    expect(await screen.findByTestId("wizard-refusal-code")).toHaveTextContent("ROUTINE");
    expect(screen.getByRole("alert")).toHaveTextContent("这份问卷核心不收。");
    expect(core.callsTo("complete_wizard")).toHaveLength(0);
    expect(onComplete).not.toHaveBeenCalled();
  });
});
