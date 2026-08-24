/**
 * AC-02, shell side: the first-run wizard finishes with everything off, and it
 * has no way to turn anything on.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import type { ConfigSnapshot } from "../core";
import { CLOSED_SNAPSHOT, CLOUD_LABEL, forbidNetwork, installFakeCore } from "../test/fakeCore";
import { Wizard } from "./Wizard";

function renderWizard(snapshot: ConfigSnapshot = CLOSED_SNAPSHOT) {
  const core = installFakeCore(snapshot);
  const onComplete = vi.fn();
  render(<Wizard snapshot={snapshot} onComplete={onComplete} />);
  return { core, onComplete };
}

describe("首次向导", () => {
  it("每一项能力在开始使用之前都是关的", async () => {
    const { core, onComplete } = renderWizard();
    const user = userEvent.setup();

    expect(screen.getByTestId("wizard-state-前台应用使用时长采集")).toHaveTextContent("关");
    expect(screen.getByTestId("wizard-state-云端深度分析")).toHaveTextContent(CLOUD_LABEL);
    expect(screen.getByTestId("wizard-state-语言模型端点")).toHaveTextContent("未填写");
    expect(screen.getByTestId("wizard-state-已授权目录")).toHaveTextContent("0 个");

    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "开始使用" }));

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
   * "不弹一堆权限" is checkable: the only tick box on the screen is the one
   * that says the user read the page. A permission checkbox added later fails
   * here before anyone has to notice it in review.
   */
  it("除了「我读过」之外没有第二个勾选框", () => {
    renderWizard();
    const boxes = screen.getAllByRole("checkbox");
    expect(boxes).toHaveLength(1);
    expect(screen.getByLabelText(/我读过上面这几行/)).toBe(boxes[0]);
  });

  it("没有勾选之前不能开始，也不会去问核心", async () => {
    const { core } = renderWizard();
    const user = userEvent.setup();

    const start = screen.getByRole("button", { name: "开始使用" });
    expect(start).toBeDisabled();
    await user.click(start);

    expect(core.callsTo("complete_wizard")).toHaveLength(0);
  });

  it("走完整个向导不碰网络", async () => {
    const attempts = forbidNetwork();
    const { onComplete } = renderWizard();
    const user = userEvent.setup();

    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "开始使用" }));

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
      ...CLOSED_SNAPSHOT,
      collect_enabled: true,
      llm_endpoint_configured: true,
      authorized_root_count: 2,
      fully_closed: false,
      open_capabilities: ["collect_enabled", "llm_endpoint"],
    });

    expect(screen.getByTestId("wizard-state-前台应用使用时长采集")).toHaveTextContent("开");
    expect(screen.getByTestId("wizard-state-语言模型端点")).toHaveTextContent("已填写");
    expect(screen.getByTestId("wizard-state-已授权目录")).toHaveTextContent("2 个");
  });
});
