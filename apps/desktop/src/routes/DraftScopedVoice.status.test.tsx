import { StrictMode } from "react";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { updateMemory } from "../core";
import { installFakeCore, type FakeCoreOptions } from "../test/fakeCore";
import { DraftScopedVoice } from "./DraftScopedVoice";

const PERSON = "11111111-1111-4111-8111-111111111111";
const MEMORY = "0192f000-0000-7000-8000-0000000000a1";
type Status = { person_id: string; occasion: "personal"; state: "active" | "absent" | "inactive" | "forgotten" | "unavailable";
  preference: { preference_id: string; revision: string; memory_id: string; register: "casual" } | null; memory_id: string | null; notice: string };
const status = (state: Status["state"]): Status => ({ person_id: PERSON, occasion: "personal", state,
  preference: state === "active" ? { preference_id: "preference", revision: "r1", memory_id: MEMORY, register: "casual" } : null,
  memory_id: state === "absent" ? null : MEMORY, notice: `核心状态说明：${state}` });
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
async function page(readingScopedRegisterStatus: () => Status | Promise<Status>) {
  const options: FakeCoreOptions & { readingScopedRegisterStatus: () => Status | Promise<Status> } = {
    scopedRecipients: [{ person_id: PERSON, identifier_hint: "synthetic-person" }], readingScopedRegisterStatus,
  };
  const fake = installFakeCore(options); const user = userEvent.setup();
  const view = render(<StrictMode><DraftScopedVoice onExit={() => undefined} /></StrictMode>);
  await user.selectOptions(await screen.findByLabelText("收件人物"), PERSON);
  await user.selectOptions(screen.getByLabelText("场合"), "personal");
  return { fake, user, ...view };
}
afterEach(() => vi.restoreAllMocks());
describe("手动读取精确范围当前偏好", () => {
  it.each(["active", "absent", "inactive", "forgotten", "unavailable"] as const)("仅点击后读取并显示%s快照，不生成或改变偏好", async (state) => {
    const { fake, user } = await page(() => status(state));
    expect(fake.callsTo("scoped_register_status")).toHaveLength(0);
    await user.click(screen.getByRole("button", { name: "查看此范围当前偏好" }));
    expect(await screen.findByText(`核心状态说明：${state}`)).toBeVisible();
    expect(fake.callsTo("scoped_register_status")[0]?.payload).toEqual({ personId: PERSON, occasion: "personal" });
    if (state === "active") {
      expect(screen.getByTestId("scoped-current-status")).toHaveTextContent("casual");
      expect(screen.getByRole("link", { name: "打开这条偏好记忆" })).toHaveAttribute("href", `#/memory?memory_id=${MEMORY}`);
    }
    for (const command of ["draft_scoped_reply", "propose_scoped_register", "confirm_scoped_register", "memory_detail"]) expect(fake.callsTo(command)).toHaveLength(0);
  });
  it.each(["scope", "source", "unmount"] as const)("查询晚到遇%s变化不展示旧状态", async (change) => {
    const pending = deferred<Status>(); const { fake, user, unmount } = await page(() => pending.promise);
    await user.click(screen.getByRole("button", { name: "查看此范围当前偏好" }));
    await waitFor(() => expect(fake.callsTo("scoped_register_status")).toHaveLength(1));
    if (change === "scope") fireEvent.change(screen.getByLabelText("场合"), { target: { value: "work" } });
    else if (change === "source") await act(async () => { await updateMemory(MEMORY, { summary: "合成资料修改" }); });
    else unmount();
    await act(async () => { pending.resolve(status("active")); });
    expect(screen.queryByTestId("scoped-current-status")).toBeNull();
  });
});
