import { invoke } from "@tauri-apps/api/core";
import { expect, it, vi } from "vitest";
import { authorizeDirectory, clearUserEndpoint, FilePlanSourceChangedError, previewPlan } from "./core";
import { aFilesView, aPlanPreview } from "./test/fakeCore";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

it("a native scan result delivered after a source mutation is rejected", async () => {
  let deliver!: (value: unknown) => void;
  vi.mocked(invoke).mockImplementation((command) => command === "preview_plan"
    ? new Promise((resolve) => { deliver = resolve; })
    : Promise.resolve(true));
  const result = previewPlan("/synthetic/root");
  const rejected = expect(result).rejects.toBeInstanceOf(FilePlanSourceChangedError);
  await clearUserEndpoint();
  deliver(aPlanPreview());
  await rejected;
});

it("pending directory authorization prevents a new scan from reaching IPC", async () => {
  let deliver!: (value: unknown) => void;
  vi.mocked(invoke).mockImplementation(() => new Promise((resolve) => { deliver = resolve; }));
  const authorize = authorizeDirectory("/synthetic/root");
  await expect(previewPlan("/synthetic/root")).rejects.toBeInstanceOf(FilePlanSourceChangedError);
  expect(vi.mocked(invoke).mock.calls.at(-1)?.[0]).toBe("authorize_directory");
  deliver(aFilesView());
  await authorize;
});
