import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import * as core from "./core";
import { aPersonSummary } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
beforeEach(() => { vi.mocked(invoke).mockReset(); });

describe("summary delivery after native completion", () => {
  it.each(["clear_user_endpoint", "correct_tie", "commit_telegram"])("does not deliver an already resolved summary when %s starts before its continuation", async (command) => {
    vi.mocked(invoke).mockImplementation((name) => Promise.resolve(name === "person_summary" ? aPersonSummary() : undefined) as Promise<never>);
    const result = core.personSummary("contact");
    const rejected = expect(result).rejects.toBeInstanceOf(core.SummarySourceChangedError);
    if (command === "clear_user_endpoint") await core.clearUserEndpoint();
    else if (command === "correct_tie") await core.correctTie("tie", "weak");
    else await core.commitTelegram("synthetic");
    await rejected;
    expect(invoke).toHaveBeenCalledWith("person_summary", { contactId: "contact" });
  });

  it("returns an unchanged native refusal without claiming that a request was cancelled", async () => {
    const refusal = { reason_code: "ROUTINE", explanation: "synthetic refusal" };
    vi.mocked(invoke).mockRejectedValue(refusal);
    await expect(core.personSummary("contact")).rejects.toBe(refusal);
  });
});
