/** Real React + core.ts continuations; only the native IPC transport is mocked. */
import { invoke } from "@tauri-apps/api/core";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import * as core from "../core";
import { anE1Plan, aProfileScreen, aTemplateDraft, DRAFT_NOTICES } from "../test/fakeCore";
import { Draft } from "./Draft";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

const TEXT = "合成联合回归原文";
const PLAN = anE1Plan();
const ORIGINAL = anE1Plan({
  preparation_id: "0192f000-0000-7000-8000-0000000000f2",
  plan_hash: "e6".repeat(32),
  placeheld_turns: 0,
  carries_exempted_original: true,
});
const RESULT = aTemplateDraft({ text: "合成端点结果", source: "user_endpoint" });
let generation: Promise<core.Draft> | null = null;
let mutation: Promise<core.ProfileScreen> | null = null;

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

function calls(command: string) {
  return vi.mocked(invoke).mock.calls.filter(([name]) => name === command);
}

async function page() {
  const mounted = render(<Draft />);
  const user = userEvent.setup();
  const input = await screen.findByLabelText("原文");
  fireEvent.change(input, { target: { value: TEXT } });
  return {
    ...mounted, user, input,
    endpoint: screen.getByRole("button", { name: "用你自己的模型端点写" }),
  };
}

async function prepared() {
  const view = await page();
  await view.user.click(view.endpoint);
  await screen.findByRole("heading", { name: "确认这一次要生成什么" });
  return view;
}

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  generation = null;
  mutation = null;
  vi.mocked(invoke).mockImplementation((command, args) => {
    let value: unknown;
    switch (command) {
      case "draft_notices": value = DRAFT_NOTICES; break;
      case "prepare_draft":
        value = (args as { includeOriginal: boolean }).includeOriginal ? ORIGINAL : PLAN;
        break;
      case "generate_draft": return (generation ?? Promise.resolve(RESULT)) as Promise<never>;
      case "correct_axis": return (mutation ?? Promise.resolve(aProfileScreen())) as Promise<never>;
      default: throw new Error(`Unexpected synthetic IPC command: ${command}`);
    }
    return Promise.resolve(value) as Promise<never>;
  });
});

describe("起草页面与真实 core wrapper 的联合交付", () => {
  it.each(["before wrapper", "between wrapper and UI"] as const)(
    "a mutation %s prevents an already successful IPC reply from appearing",
    async (timing) => {
      const response = deferred<core.Draft>();
      const changed = deferred<core.ProfileScreen>();
      generation = response.promise;
      mutation = changed.promise;
      const view = await prepared();
      await view.user.click(screen.getByRole("button", { name: "确认，开始生成" }));
      expect(calls("generate_draft")).toHaveLength(1);

      let change: Promise<core.ProfileScreen> | undefined;
      if (timing === "between wrapper and UI") {
        // core.ts registered its transport .then when the click started IPC.
        // This observer runs after that wrapper check, but before fulfilling
        // the wrapper promise resumes the component's await continuation.
        void response.promise.then(() => { change = core.correctAxis("axis", "position"); });
      }
      await act(async () => {
        response.resolve(RESULT);
        if (timing === "before wrapper") change = core.correctAxis("axis", "position");
      });

      expect(calls("correct_axis")).toHaveLength(1);
      expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
      expect(screen.queryByDisplayValue(RESULT.text)).toBeNull();
      expect(screen.queryByTestId("draft-recovery")).toBeNull();
      expect(screen.queryByTestId("draft-source-changed")).toBeNull();
      expect(view.input).toHaveValue(TEXT);
      await act(async () => { changed.resolve(aProfileScreen()); await change; });
      expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    },
  );

  it("a displayed result is cleared by the real mutation wrapper before its IPC completes", async () => {
    const view = await prepared();
    await view.user.click(screen.getByRole("button", { name: "确认，开始生成" }));
    await screen.findByDisplayValue(RESULT.text);
    const changed = deferred<core.ProfileScreen>();
    mutation = changed.promise;
    let change!: Promise<core.ProfileScreen>;
    act(() => { change = core.correctAxis("axis", "position"); });
    expect(screen.queryByRole("heading", { name: "草稿" })).toBeNull();
    expect(view.input).toHaveValue(TEXT);
    expect(calls("correct_axis")).toHaveLength(1);
    await act(async () => { changed.resolve(aProfileScreen()); await change; });
  });

  it("pending mutation rejects new preparation without invoking native prepare, then recovers", async () => {
    const view = await page();
    const changed = deferred<core.ProfileScreen>();
    mutation = changed.promise;
    let change!: Promise<core.ProfileScreen>;
    act(() => { change = core.correctAxis("axis", "position"); });
    await view.user.click(view.endpoint);
    expect(calls("prepare_draft")).toHaveLength(0);
    expect(screen.queryByRole("heading", { name: "确认这一次要生成什么" })).toBeNull();
    expect(screen.queryByTestId("draft-recovery")).toBeNull();
    expect(await screen.findByTestId("draft-source-changed")).toHaveTextContent("请等相关操作完成后重新准备");
    expect(screen.getByTestId("draft-source-changed")).toHaveAttribute("role", "status");
    expect(screen.queryByTestId("refusal-code")).toBeNull();
    expect(view.input).toHaveValue(TEXT);
    await act(async () => { changed.resolve(aProfileScreen()); await change; });
    expect(screen.queryByTestId("draft-source-changed")).toBeNull();
    await view.user.click(view.endpoint);
    await screen.findByRole("heading", { name: "确认这一次要生成什么" });
    expect(calls("prepare_draft")).toHaveLength(1);
  });

  it("mutation between the approval click and IPC start invalidates the admitted approval", async () => {
    const view = await prepared();
    const changed = deferred<core.ProfileScreen>();
    mutation = changed.promise;
    let change!: Promise<core.ProfileScreen>;
    const approve = screen.getByRole("button", { name: "确认，开始生成" });
    act(() => {
      approve.click();
      change = core.correctAxis("axis", "position");
    });
    await waitFor(() => expect(view.endpoint).toBeEnabled());
    expect(calls("generate_draft")).toHaveLength(0);
    expect(screen.queryByRole("button", { name: "确认，开始生成" })).toBeNull();
    expect(screen.queryByTestId("draft-recovery")).toBeNull();
    await act(async () => { changed.resolve(aProfileScreen()); await change; });
  });

  it("normal preparation, one-shot original confirmation and generation retain their exact IPC arguments", async () => {
    const view = await prepared();
    await view.user.click(screen.getByRole("button", { name: "这一条按原文带上" }));
    await screen.findByText("有一段是你二次确认过、按原文带上的。");
    expect(calls("prepare_draft")).toEqual([
      ["prepare_draft", { pasted: TEXT, includeOriginal: false }],
      ["prepare_draft", { pasted: TEXT, includeOriginal: true }],
    ]);
    await view.user.click(screen.getByRole("button", { name: "确认，开始生成" }));
    await screen.findByDisplayValue(RESULT.text);
    expect(calls("generate_draft")).toEqual([["generate_draft", {
      approval: { preparation_id: ORIGINAL.preparation_id, plan_hash: ORIGINAL.plan_hash },
    }]]);
    expect(screen.queryByTestId("draft-recovery")).toBeNull();
    expect(view.endpoint).toBeEnabled();
  });
});
