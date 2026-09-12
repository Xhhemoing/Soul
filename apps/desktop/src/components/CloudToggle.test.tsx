/**
 * AC-22, shell side: the cloud switch is on screen, it reads 尚未启用, and
 * pressing it leaves it reading 尚未启用 without touching the network.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { CLOSED_SNAPSHOT, CLOUD_LABEL, forbidNetwork, installFakeCore } from "../test/fakeCore";
import { CloudToggle } from "./CloudToggle";

describe("云端开关", () => {
  it("可见，而且写着尚未启用", () => {
    installFakeCore();
    render(<CloudToggle notice={CLOSED_SNAPSHOT.cloud} />);

    const toggle = screen.getByRole("switch");
    expect(toggle).toBeVisible();
    expect(toggle).toHaveAttribute("aria-checked", "false");
    expect(screen.getByTestId("cloud-state")).toHaveTextContent(CLOUD_LABEL);
  });

  it("点了还是尚未启用，而且一个请求都没发出去", async () => {
    const attempts = forbidNetwork();
    const core = installFakeCore();
    const user = userEvent.setup();
    render(<CloudToggle notice={CLOSED_SNAPSHOT.cloud} />);

    await user.click(screen.getByRole("switch"));

    expect(await screen.findByTestId("cloud-press-result")).toHaveTextContent(CLOUD_LABEL);
    expect(screen.getByTestId("cloud-state")).toHaveTextContent(CLOUD_LABEL);
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "false");
    expect(attempts).toEqual([]);

    // The request that was made asked for "on". The answer not depending on it
    // is the claim; a component that never asked would prove nothing.
    expect(core.callsTo("cloud_toggle")).toHaveLength(1);
    expect(core.callsTo("cloud_toggle")[0]?.payload).toEqual({ requestedOn: true });
  });

  it("连点五次也不会有任何一次把它打开", async () => {
    const attempts = forbidNetwork();
    const core = installFakeCore();
    const user = userEvent.setup();
    render(<CloudToggle notice={CLOSED_SNAPSHOT.cloud} />);

    const toggle = screen.getByRole("switch");
    for (let press = 0; press < 5; press += 1) {
      await user.click(toggle);
    }

    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "false");
    expect(screen.getByTestId("cloud-state")).toHaveTextContent(CLOUD_LABEL);
    expect(core.callsTo("cloud_toggle")).toHaveLength(5);
    for (const call of core.callsTo("cloud_toggle")) {
      expect(call.payload).toEqual({ requestedOn: true });
    }
    expect(attempts).toEqual([]);
  });

  it("说明文字来自核心，不是界面自己编的", () => {
    installFakeCore();
    render(<CloudToggle notice={CLOSED_SNAPSHOT.cloud} />);
    expect(screen.getByText(CLOSED_SNAPSHOT.cloud.explanation)).toBeVisible();
  });
});
