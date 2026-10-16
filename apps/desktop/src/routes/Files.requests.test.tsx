import { act, fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, expect, it, vi } from "vitest";
import * as core from "../core";
import { aFilesView, aPlanPreview } from "../test/fakeCore";
import { Files } from "./Files";

vi.mock("../core", () => ({
  authorizeDirectory: vi.fn(), filesView: vi.fn(), previewPlan: vi.fn(), subscribeDraftSource: vi.fn(),
  FilePlanSourceChangedError: class FilePlanSourceChangedError extends Error {},
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => { resolve = done; });
  return { promise, resolve };
}

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(core.filesView).mockResolvedValue(aFilesView());
  vi.mocked(core.subscribeDraftSource).mockReturnValue(() => {});
});

it("a late authorization keeps edited input and double click has one owner", async () => {
  const pending = deferred<core.FilesView>();
  vi.mocked(core.authorizeDirectory).mockReturnValue(pending.promise);
  render(<Files />);
  const input = await screen.findByLabelText("目录完整路径");
  fireEvent.change(input, { target: { value: "/synthetic/A" } });
  const button = screen.getByRole("button", { name: "授权这个目录" });
  fireEvent.click(button);
  fireEvent.click(button);
  fireEvent.change(input, { target: { value: "/synthetic/B" } });
  expect(button).toBeDisabled();
  await act(async () => { pending.resolve(aFilesView()); });
  expect(input).toHaveValue("/synthetic/B");
  expect(button).toBeEnabled();
  expect(core.authorizeDirectory).toHaveBeenCalledTimes(1);
});

it("an edited scan stays occupied until settlement and cannot display its plan", async () => {
  const pending = deferred<core.PlanPreview>();
  vi.mocked(core.previewPlan).mockReturnValue(pending.promise);
  render(<Files />);
  const scan = await screen.findByRole("button", { name: "看整理计划" });
  fireEvent.click(scan);
  fireEvent.change(screen.getByLabelText("目录完整路径"), { target: { value: "/synthetic/new" } });
  expect(scan).toBeDisabled();
  await act(async () => { pending.resolve(aPlanPreview()); });
  expect(screen.queryByRole("heading", { name: "整理计划" })).toBeNull();
  expect(scan).toBeEnabled();
});

it("an unmounted scan cannot publish into a remounted route", async () => {
  const pending = deferred<core.PlanPreview>();
  vi.mocked(core.previewPlan).mockReturnValue(pending.promise);
  const old = render(<Files />);
  fireEvent.click(await screen.findByRole("button", { name: "看整理计划" }));
  old.unmount();
  render(<Files />);
  await screen.findByRole("button", { name: "看整理计划" });
  await act(async () => { pending.resolve(aPlanPreview()); });
  expect(screen.queryByRole("heading", { name: "整理计划" })).toBeNull();
});

it("source invalidation between delivery and the route continuation rejects the old plan", async () => {
  let changed!: (revision: symbol) => void;
  vi.mocked(core.subscribeDraftSource).mockImplementation((listener) => {
    changed = listener;
    return () => {};
  });
  const pending = deferred<core.PlanPreview>();
  vi.mocked(core.previewPlan).mockReturnValue(pending.promise);
  render(<Files />);
  fireEvent.click(await screen.findByRole("button", { name: "看整理计划" }));
  await act(async () => {
    pending.resolve(aPlanPreview());
    changed(Symbol("source changed"));
  });
  expect(screen.queryByRole("heading", { name: "整理计划" })).toBeNull();
});

it("a stale initial roots read cannot overwrite a completed authorization", async () => {
  const initial = deferred<core.FilesView>();
  vi.mocked(core.filesView).mockReturnValue(initial.promise);
  const authorized = { ...aFilesView(), roots: [{ path: "/synthetic/new-root" }] };
  vi.mocked(core.authorizeDirectory).mockResolvedValue(authorized);
  render(<Files />);
  fireEvent.change(screen.getByLabelText("目录完整路径"), { target: { value: "/synthetic/new-root" } });
  fireEvent.click(screen.getByRole("button", { name: "授权这个目录" }));
  await screen.findByText("/synthetic/new-root");
  await act(async () => { initial.resolve({ ...aFilesView(), roots: [] }); });
  expect(screen.getByText("/synthetic/new-root")).toBeVisible();
});

it("a synchronous scan transport failure releases the physical UI slot", async () => {
  vi.mocked(core.previewPlan).mockImplementation(() => { throw new Error("synthetic transport failure"); });
  render(<Files />);
  const button = await screen.findByRole("button", { name: "看整理计划" });
  fireEvent.click(button);
  await screen.findByRole("alert");
  expect(button).toBeEnabled();
});
