/**
 * The draft page as a product surface: paste, generate, read. Nothing here
 * sends anything, and the core's own sentences are what the screen shows.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import {
  CLOSED_DRAFT,
  DRAFT_NEVER_SENT_EXPLANATION,
  EMPTY_PASTE_EXPLANATION,
  forbidNetwork,
  installFakeCore,
  TEMPLATE_ROUTE_LABEL,
} from "../test/fakeCore";
import { Draft } from "./Draft";

describe("起草页", () => {
  it("粘贴之后生成的是核心给的草稿、路线和说明，一个字都没改", async () => {
    const attempts = forbidNetwork();
    const core = installFakeCore();
    render(<Draft />);

    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "粘贴" }), "周末有空一起吃饭吗");
    await user.click(screen.getByRole("button", { name: "生成草稿" }));

    expect(await screen.findByTestId("draft-text")).toHaveTextContent(CLOSED_DRAFT.text);
    expect(screen.getByTestId("draft-route")).toHaveTextContent(TEMPLATE_ROUTE_LABEL);
    expect(screen.getByTestId("draft-notice").textContent).toBe(DRAFT_NEVER_SENT_EXPLANATION);
    expect(screen.getByTestId("draft-placeheld")).toHaveTextContent(
      `占位 ${String(CLOSED_DRAFT.placeheld_turns)} 段`,
    );
    expect(core.callsTo("draft_view")).toHaveLength(1);
    expect(attempts).toEqual([]);
  });

  it("空粘贴时核心的拒绝语上屏，而不是自己编一句", async () => {
    const attempts = forbidNetwork();
    installFakeCore();
    render(<Draft />);

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "生成草稿" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(EMPTY_PASTE_EXPLANATION);
    expect(screen.queryByTestId("draft-result")).toBeNull();
    expect(attempts).toEqual([]);
  });

  it("整页没有任何发送或递出的控件", () => {
    const attempts = forbidNetwork();
    installFakeCore();
    render(<Draft />);

    const names = screen.queryAllByRole("button").map((button) => button.textContent ?? "");
    expect(names).toEqual(["生成草稿", "清空"]);
    expect(screen.queryByRole("button", { name: /发送|send|submit|deliver/i })).toBeNull();
    expect(screen.queryByRole("link")).toBeNull();
    expect(attempts).toEqual([]);
  });
});
