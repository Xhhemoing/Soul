/**
 * 自传记忆: writing one, editing one, and the two-step forget.
 *
 * The forget is what most of this file is about. It is the one destructive
 * thing v0.1 does — D15's definition of deleting, which is destroying the
 * content key — so the tests below care that reading the price is not paying
 * it, that paying it echoes the core's own answer back unchanged, and that a
 * confirmation naming a preview the core is not holding destroys nothing.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { forgetMemory } from "../core";
import { Memory } from "./Memory";
import {
  aForgetPreview,
  aMemoryDetail,
  aMemoryList,
  forbidNetwork,
  installFakeCore,
  PREVIEW_ID,
  type FakeCoreOptions,
} from "../test/fakeCore";

const MEMORY_ID = "0192f000-0000-7000-8000-0000000000a1";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Memory />);
  await screen.findByRole("heading", { name: "记一条" });
  return core;
}

describe("自传记忆页", () => {
  it("库里没有记忆的时候，说清楚为什么是空的", async () => {
    await open();

    expect(screen.getByTestId("no-memories")).toHaveTextContent("还没有写过记忆");
    expect(screen.getByTestId("forget-notice")).toHaveTextContent("不可撤销");
    // PRODUCT_LOCK promises no SSD physical erase and that the UI says so, so
    // the sentence on screen has to name the disk it does not scrub.
    expect(screen.getByTestId("forget-notice")).toHaveTextContent("磁盘块");
    expect(screen.getByTestId("forget-notice")).toHaveTextContent("SSD");
  });

  /**
   * The other limit, which is not the SSD one: a forget writes. It deletes the
   * content-key and sealed-blob rows, lays a tombstone, orphans the inferences
   * and appends an audit record, all inside Soul's own encrypted database — so
   * the notice may promise the scope of what it touches and may not promise
   * that no file changes.
   */
  it("遗忘说明不声称零写入，而是写清楚动的是 Soul 自己的库", async () => {
    await open();

    const notice = screen.getByTestId("forget-notice");
    expect(notice).toHaveTextContent("数据目录以外的任何文件");
    expect(notice).toHaveTextContent("Soul 自己的加密库要写");
    expect(notice).toHaveTextContent("审计记录");
    expect(notice.textContent).not.toContain("也不写任何文件");
  });

  it("写一条会把用户填的原样交给核心", async () => {
    const core = await open();
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("一句话说清是什么"), "搬家那天");
    await user.type(screen.getByLabelText("再多写几句"), "下午三点交的钥匙。");
    await user.click(screen.getByRole("button", { name: "存下来" }));

    expect(core.callsTo("create_memory")[0]?.payload).toEqual({
      memory: {
        memory_type: "episodic",
        title: "搬家那天",
        summary: "下午三点交的钥匙。",
      },
    });
  });

  /**
   * A list view is handed digests, not bodies. What is on screen is the kind,
   * two character counts and where the memory is on the way to being
   * forgotten; opening it is a separate command.
   */
  it("列表只显示字数和状态，正文要另外打开", async () => {
    const core = await open({ memories: () => aMemoryList() });
    const user = userEvent.setup();

    const row = screen.getByTestId(`memory-${MEMORY_ID}`);
    expect(row).toHaveTextContent("标题 6 字，正文 24 字");
    expect(row.textContent ?? "").not.toContain("搬家那天");

    await user.click(within(row).getByRole("button", { name: "打开" }));

    expect(core.callsTo("memory_detail")[0]?.payload).toEqual({ memoryId: MEMORY_ID });
    expect(await screen.findByDisplayValue("搬家那天")).toBeVisible();
  });

  it("改一条之后仍然是同一个遗忘单位", async () => {
    const core = await open({ memories: () => aMemoryList() });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "打开",
    }));
    const title = await screen.findByDisplayValue("搬家那天");
    await user.clear(title);
    await user.type(title, "交钥匙那天");
    await user.click(screen.getByRole("button", { name: "改好了" }));

    expect(core.callsTo("update_memory")[0]?.payload).toEqual({
      memoryId: MEMORY_ID,
      change: {
        memory_type: "episodic",
        title: "交钥匙那天",
        summary: "下午三点交的钥匙，晚上在新厨房煮了面。",
      },
    });
    expect(screen.getByTestId("edit-key")).toHaveTextContent("同一把内容密钥");
  });

  /**
   * The heart of it: asking what a forget costs must not be the forget. The
   * assertion is on the commands, because a screen that looked right while
   * sending `forget_memory` would still have destroyed something.
   */
  it("看影响面不等于遗忘：只问价，什么都没销毁", async () => {
    const core = await open({ memories: () => aMemoryList() });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "看遗忘会影响什么",
    }));

    expect(await screen.findByTestId("forget-preview")).toHaveTextContent("要销毁 1 把内容密钥");
    expect(screen.getByTestId("preview-destroys-nothing")).toHaveTextContent("没有销毁任何东西");
    expect(core.callsTo("preview_forget")).toHaveLength(1);
    expect(core.callsTo("forget_memory")).toHaveLength(0);
  });

  it("先留着就是不做，核心那边一次遗忘也没有发生", async () => {
    const core = await open({ memories: () => aMemoryList() });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "看遗忘会影响什么",
    }));
    await screen.findByTestId("forget-preview");
    await user.click(screen.getByRole("button", { name: "先留着" }));

    expect(screen.queryByTestId("forget-preview")).toBeNull();
    expect(core.callsTo("forget_memory")).toHaveLength(0);
  });

  /** Forgetting is allowed. What it sends back is the core's own preview id. */
  it("确认之后才遗忘，交回去的是核心发的那份预览编号", async () => {
    const core = await open({ memories: () => aMemoryList() });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "看遗忘会影响什么",
    }));
    await screen.findByTestId("forget-preview");
    await user.click(screen.getByRole("button", { name: "就按上面这些，遗忘它" }));

    expect(core.callsTo("forget_memory")[0]?.payload).toEqual({
      confirmation: { preview_id: PREVIEW_ID, memory_id: MEMORY_ID },
    });
    // One press, one forget. The core spends the preview it matched, so a
    // second call would reach nothing — but a page that sent two would already
    // have destroyed something by the time the second one was refused, and the
    // user would be reading a denial for an act that ran.
    expect(core.callsTo("forget_memory")).toHaveLength(1);
    expect(await screen.findByTestId("forget-receipt")).toHaveTextContent("销毁了 1 把内容密钥");
    expect(screen.getByTestId("receipt-matched")).toHaveTextContent("和你看过的那份预览一致");
  });

  /**
   * The other half of the same contract, from the screen's side: once the
   * forget has run there is nothing left to press, and the confirmation the
   * page already spent buys nothing if it is sent again.
   *
   * The replay is sent through `core.ts` rather than through the page, because
   * the page correctly offers no way to do it — which is exactly why the
   * double has to refuse it on its own. A fake that answered a second receipt
   * would let a future 记忆 page keep the panel up after a success and look
   * right in every test here.
   */
  it("遗忘做完之后，同一份确认再交一次买不到第二次销毁", async () => {
    const core = await open({ memories: () => aMemoryList() });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "看遗忘会影响什么",
    }));
    await screen.findByTestId("forget-preview");
    await user.click(screen.getByRole("button", { name: "就按上面这些，遗忘它" }));
    await screen.findByTestId("forget-receipt");

    expect(screen.queryByTestId("forget-preview")).toBeNull();
    expect(screen.queryByRole("button", { name: "就按上面这些，遗忘它" })).toBeNull();
    await expect(
      forgetMemory({ preview_id: PREVIEW_ID, memory_id: MEMORY_ID }),
    ).rejects.toMatchObject({ reason_code: "PLAN_HASH_MISMATCH" });
    expect(core.callsTo("preview_forget")).toHaveLength(1);
  });

  /**
   * The core holds one preview and refuses anything else, so a confirmation
   * built out of stale numbers destroys nothing. The screen shows the refusal
   * rather than a receipt.
   */
  it("对不上预览的确认会被核心挡下来，屏幕上照实说", async () => {
    const core = await open({
      memories: () => aMemoryList(),
      pricing: () => aForgetPreview({ preview_id: "0192f000-0000-7000-8000-0000000000ff" }),
    });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "看遗忘会影响什么",
    }));
    await screen.findByTestId("forget-preview");
    await user.click(screen.getByRole("button", { name: "就按上面这些，遗忘它" }));

    expect(await screen.findByTestId("memory-refusal-code")).toHaveTextContent(
      "PLAN_HASH_MISMATCH",
    );
    expect(screen.queryByTestId("forget-receipt")).toBeNull();
    expect(core.callsTo("forget_memory")).toHaveLength(1);
  });

  /**
   * `Session::forget_memory` matches before it takes, so a refused
   * confirmation costs the click and not the preview: the price the user read
   * is still the one the core is holding, and the panel quoting it has to
   * still be on screen. A page that cleared it would make one wrong id the
   * reason to walk the irreversible screen a second time, and a second walk
   * through a screen like this one gets clicked rather than read.
   */
  it("被挡下来之后，用户读过的那份预览还在屏幕上，核心也还留着它", async () => {
    const core = await open({
      memories: () => aMemoryList(),
      pricing: () => aForgetPreview({ preview_id: "0192f000-0000-7000-8000-0000000000ff" }),
    });
    const user = userEvent.setup();

    await user.click(within(screen.getByTestId(`memory-${MEMORY_ID}`)).getByRole("button", {
      name: "看遗忘会影响什么",
    }));
    await screen.findByTestId("forget-preview");
    await user.click(screen.getByRole("button", { name: "就按上面这些，遗忘它" }));
    await screen.findByTestId("memory-refusal-code");

    // The same panel, quoting the same numbers, with the same button under it.
    expect(screen.getByTestId("forget-preview")).toHaveTextContent("要销毁 1 把内容密钥");
    expect(screen.getByTestId("forget-preview-id")).toHaveTextContent(
      "0192f000-0000-7000-8000-0000000000ff",
    );
    expect(screen.getByRole("button", { name: "就按上面这些，遗忘它" })).toBeEnabled();
    // And the page did not quietly re-price it behind the refusal, which would
    // be the second walk in everything but appearance.
    expect(core.callsTo("preview_forget")).toHaveLength(1);

    // The refusal spent nothing on the core's side either: the preview it
    // issued is still there, and the confirmation the user meant to send goes
    // through.
    const receipt = await forgetMemory({ preview_id: PREVIEW_ID, memory_id: MEMORY_ID });
    expect(receipt.matched_preview).toBe(true);
  });

  /** A tombstone can be read about and not reopened. */
  it("已经遗忘的那一条不能再打开，也不能再遗忘一次", async () => {
    await open({
      memories: () =>
        aMemoryList({
          memories: aMemoryList().memories.map((row) => ({ ...row, forget_state: "forgotten" })),
        }),
    });

    const row = screen.getByTestId(`memory-${MEMORY_ID}`);
    expect(row).toHaveTextContent("已遗忘，只剩墓碑");
    expect(within(row).getByRole("button", { name: "打开" })).toBeDisabled();
    expect(within(row).getByRole("button", { name: "看遗忘会影响什么" })).toBeDisabled();
  });

  /**
   * Forgetting destroys a key; it does not touch a file, and AC-27 is about
   * the other kind of act. Nothing on this page offers to write one.
   */
  it("这一页上没有一个按钮是发送或写文件", async () => {
    const attempts = forbidNetwork();
    await open({ memories: () => aMemoryList(), opening: () => aMemoryDetail() });

    for (const button of screen.queryAllByRole("button")) {
      expect(button.textContent ?? "").not.toMatch(/发送|发出|导出|保存到|写入文件|移动|重命名/);
    }
    expect(attempts).toEqual([]);
  });
});
