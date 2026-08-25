/**
 * The file-plan screen: what it asks the core for, what it shows, and the
 * button it must never have.
 *
 * `soul-fileplan` proves the scan reads and never writes, and `soulcore`
 * proves the command surface has no `execute`. What only this side can check
 * is that the screen in front of a person does not offer to carry the plan
 * out, and does not soften the sentence that says why.
 */

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { Files } from "./Files";
import {
  aFilesView,
  aPlanPreview,
  forbidNetwork,
  installFakeCore,
  NO_ROOTS,
  READ_ONLY_NOTICE,
  type FakeCoreOptions,
} from "../test/fakeCore";

const ROOT = "/home/li/下载";

async function open(options: FakeCoreOptions = {}) {
  const core = installFakeCore(options);
  render(<Files />);
  await screen.findByRole("heading", { name: "已授权的目录" });
  return core;
}

/** Every button on screen, whatever state it is in. */
function buttonWords(): string[] {
  return screen.queryAllByRole("button").map((button) => button.textContent ?? "");
}

describe("文件计划页", () => {
  it("还没有授权目录的时候，说清楚 Soul 现在读不到任何文件", async () => {
    await open();

    expect(screen.getByTestId("no-roots")).toHaveTextContent("还没有授权任何目录");
    expect(screen.getByTestId("read-only-notice")).toHaveTextContent(READ_ONLY_NOTICE);
  });

  it("授权一个目录之后，目录出现在列表里", async () => {
    const core = await open({ authorizing: () => aFilesView() });
    const user = userEvent.setup();

    await user.type(screen.getByLabelText("目录完整路径"), ROOT);
    await user.click(screen.getByRole("button", { name: "授权这个目录" }));

    expect(await screen.findByText(ROOT)).toBeVisible();
    expect(core.callsTo("authorize_directory")[0]?.payload).toEqual({ path: ROOT });
  });

  /**
   * The plan, as the user reads it: what would move, what would stay, why each
   * one, and the hash that says which directory this describes.
   */
  it("扫描之后给出移动清单、原地不动清单和计划哈希", async () => {
    await open({ files: aFilesView() });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "看整理计划" }));

    const moves = await screen.findByTestId("plan-moves");
    expect(within(moves).getByText("表格/预算.csv")).toBeVisible();
    expect(within(moves).getByText("图片/photo.jpg")).toBeVisible();

    const left = screen.getByTestId("plan-left-alone");
    expect(within(left).getByText("mystery.qqq")).toBeVisible();
    expect(left).toHaveTextContent("认不出这是什么类型的文件");

    expect(screen.getByTestId("plan-hash")).toHaveTextContent("b3b3");
    expect(screen.getByTestId("disk-unchanged")).toHaveTextContent("没有改动任何东西");
    expect(screen.getByTestId("plan-read-only")).toHaveTextContent(READ_ONLY_NOTICE);
  });

  /**
   * The absence WP11 is about, asserted at the moment somebody would add the
   * button: with a plan on screen and an obvious next step.
   */
  it("有了计划之后，页面上仍然没有执行、应用或移动的按钮", async () => {
    await open({ files: aFilesView() });
    const user = userEvent.setup();

    for (const words of buttonWords()) {
      expect(words).not.toMatch(/执行|应用|移动|重命名|删除|撤销/);
    }

    await user.click(screen.getByRole("button", { name: "看整理计划" }));
    await screen.findByTestId("plan-moves");

    for (const words of buttonWords()) {
      expect(words).not.toMatch(/执行|应用|移动|重命名|删除|撤销/);
    }
    // And the screen says so in the core's own words rather than by omission.
    expect(screen.getByTestId("plan-read-only")).toHaveTextContent("v0.1.1");
  });

  it("被拒绝的路径给出理由码和一句话，而不是空白的计划", async () => {
    installFakeCore({
      files: NO_ROOTS,
      authorizing: () => {
        throw { reason_code: "CONSENT_MISSING", explanation: "`/etc` 不在任何已授权目录里" };
      },
    });
    const user = userEvent.setup();
    render(<Files />);

    await user.type(await screen.findByLabelText("目录完整路径"), "/etc");
    await user.click(screen.getByRole("button", { name: "授权这个目录" }));

    const alert = await screen.findByRole("alert");
    expect(within(alert).getByTestId("files-refusal-code")).toHaveTextContent("CONSENT_MISSING");
    expect(alert).toHaveTextContent("不在任何已授权目录里");
  });

  it("找不到的目录被列出来，而不是从名单里消失", async () => {
    await open({
      files: aFilesView({
        unavailable_roots: [{ path: "/home/li/搬走了", explanation: "读不到：可能不存在" }],
      }),
    });

    const gone = screen.getByTestId("unavailable-roots");
    expect(within(gone).getByText("/home/li/搬走了")).toBeVisible();
    expect(gone).toHaveTextContent("读不到");
  });

  it("整个扫描过程里界面没有碰过网络", async () => {
    const attempts = forbidNetwork();
    await open({ files: aFilesView(), planning: () => aPlanPreview() });
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "看整理计划" }));
    await screen.findByTestId("plan-moves");

    expect(attempts).toEqual([]);
  });
});
