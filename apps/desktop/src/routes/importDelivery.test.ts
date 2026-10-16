import { describe, expect, it, vi } from "vitest";
import { anImportReceipt } from "../test/fakeCore";
import { ImportCommitBlockedError, ImportCommitUnknownError, ImportDelivery } from "./importDelivery";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

describe("import acknowledgement ownership", () => {
  it("owns one physical call before invoking even a reentrant factory", async () => {
    const delivery = new ImportDelivery();
    const pending = deferred<ReturnType<typeof anImportReceipt>>();
    const duplicate = vi.fn(async () => anImportReceipt());
    let rejected: Promise<void> | undefined;
    const first = delivery.run("soul-import-v1", () => {
      rejected = expect(delivery.run("telegram-desktop", duplicate)).rejects.toBeInstanceOf(ImportCommitBlockedError);
      return pending.promise;
    });
    expect(delivery.snapshot().status).toBe("pending");
    expect(duplicate).not.toHaveBeenCalled();
    pending.resolve(anImportReceipt());
    await first;
    await rejected;
    expect(delivery.snapshot().status).toBe("ready");
  });

  it("does not release work when the old observer leaves; a new observer receives the acknowledgement", async () => {
    const delivery = new ImportDelivery();
    const pending = deferred<ReturnType<typeof anImportReceipt>>();
    const old = vi.fn();
    const stop = delivery.subscribe(old);
    const work = delivery.run("soul-import-v1", () => pending.promise);
    stop();
    const fresh = vi.fn();
    delivery.subscribe(fresh);
    expect(fresh).toHaveBeenLastCalledWith({ status: "pending", receipt: null });
    pending.resolve(anImportReceipt());
    await work;
    expect(old).toHaveBeenCalledTimes(2);
    expect(fresh).toHaveBeenLastCalledWith({ status: "ready", receipt: anImportReceipt() });
  });

  it("keeps unknown across observers and blocks all formats without storing the failure text", async () => {
    const delivery = new ImportDelivery();
    const pending = deferred<ReturnType<typeof anImportReceipt>>();
    const work = delivery.run("soul-import-v1", () => pending.promise);
    const failure = expect(work).rejects.toBeInstanceOf(ImportCommitUnknownError);
    pending.reject(new Error("PRIVATE PATH / EXPORTED BODY / CREDENTIAL"));
    await failure;
    const observer = vi.fn();
    delivery.subscribe(observer);
    expect(observer).toHaveBeenLastCalledWith({ status: "unknown", receipt: null });
    const factory = vi.fn(async () => anImportReceipt());
    await expect(delivery.run("telegram-desktop", factory)).rejects.toBeInstanceOf(ImportCommitBlockedError);
    expect(factory).not.toHaveBeenCalled();
    expect(JSON.stringify(delivery.snapshot())).not.toMatch(/PRIVATE|PATH|BODY|CREDENTIAL/);
  });

  it("a structured core refusal permits one explicit new attempt", async () => {
    const delivery = new ImportDelivery();
    const refusal = { reason_code: "ROUTINE", explanation: "合成事务已回滚。" };
    await expect(delivery.run("soul-import-v1", () => Promise.reject(refusal))).rejects.toEqual(refusal);
    expect(delivery.snapshot()).toEqual({ status: "ready", receipt: null });
    const retry = vi.fn(async () => anImportReceipt());
    await delivery.run("soul-import-v1", retry);
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it.each([
    { source: "telegram-desktop" }, { contacts_created: -1 }, { contacts_matched: 0.5 },
    { events_written: NaN }, { evidence_written: undefined },
    { messages_with_injection_markers: Number.MAX_SAFE_INTEGER + 1 }, { ties_rebuilt: "4" }, { notice: null },
  ])("does not acknowledge an invalid or mismatched receipt: %j", async (change) => {
    const delivery = new ImportDelivery();
    const value = { ...anImportReceipt(), ...change } as ReturnType<typeof anImportReceipt>;
    await expect(delivery.run("soul-import-v1", async () => value)).rejects.toBeInstanceOf(ImportCommitUnknownError);
    expect(delivery.snapshot()).toEqual({ status: "unknown", receipt: null });
  });

  it("observer failure does not alter the received commit or prevent other observers settling", async () => {
    const delivery = new ImportDelivery();
    delivery.subscribe(() => { throw new Error("broken observer"); });
    const observer = vi.fn();
    delivery.subscribe(observer);
    await delivery.run("soul-import-v1", async () => anImportReceipt());
    expect(observer).toHaveBeenLastCalledWith({ status: "ready", receipt: anImportReceipt() });
    // The exact acknowledged fields are copied, rather than arbitrary metadata.
    await delivery.run("soul-import-v1", async () => ({ ...anImportReceipt(), text: "PRIVATE" }));
    expect(JSON.stringify(delivery.snapshot())).not.toContain("PRIVATE");
  });

  it("hides an older success while another operation is pending or unknown", async () => {
    const delivery = new ImportDelivery();
    await delivery.run("soul-import-v1", async () => anImportReceipt());
    const pending = deferred<ReturnType<typeof anImportReceipt>>();
    const work = delivery.run("telegram-desktop", () => pending.promise);
    expect(delivery.snapshot().receipt).toBeNull();
    const failure = expect(work).rejects.toBeInstanceOf(ImportCommitUnknownError);
    pending.reject("transport disconnected");
    await failure;
    expect(delivery.snapshot().receipt).toBeNull();
  });
});
