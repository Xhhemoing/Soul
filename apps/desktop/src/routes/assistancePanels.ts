import { useState } from "react";
import { asRefusal } from "../refusal";
import { assistanceWrites, useAssistanceWrites } from "./assistanceRequests";
import { useAssistanceRequestOwner } from "./AssistanceWork";
import { isDraftRefusal } from "./draftRequests";

/** The payload and its owner are one state update, before any render can use it. */
export function useAssistancePanel<T>(isCurrent: () => boolean = () => true, record?: string) {
  const owner = useAssistanceRequestOwner(); const writes = useAssistanceWrites();
  const [stored, setStored] = useState<{ readonly value: T; readonly owner: { readonly address: typeof owner.address; readonly source: symbol } } | null>(null);
  const [busy, setBusy] = useState(false); const [error, setError] = useState<string | null>(null); const [notice, setNotice] = useState<string | null>(null);
  const value = stored !== null && owner.ownsPayload(stored.owner) ? stored.value : null;
  const run = (work: () => Promise<T>, write = false, success: string | null = null): void => {
    if (!owner.current() || !isCurrent() || (write && (writes.busy || writes.unknown))) return;
    const ticket = owner.begin(); if (ticket === null) return;
    const slot = write ? assistanceWrites.begin(record) : null;
    if (write && slot === null) { owner.finish(ticket); return; }
    const recovery = !write && record !== undefined ? assistanceWrites.recovery(record) : null;
    setBusy(true); setError(null); setNotice(null); let started = false; let unknown = false;
    const current = (): boolean => owner.owns(ticket) && isCurrent();
    void Promise.resolve().then(async () => {
      if (!current()) return;
      started = true; const result = await work();
      if (current()) {
        // The current ticket checks address, mount and source before a
        // relevant readback can recover a previous unknown write. A new
        // source generation may explicitly re-read the same stable record.
        if (!write && record !== undefined) assistanceWrites.checked(record, recovery);
        setStored({ value: result, owner: { address: owner.address, source: owner.source } }); setNotice(success);
      }
    }).catch((problem: unknown) => {
      unknown = write && started && !isDraftRefusal(problem);
      if (current()) { setStored(null); setError(unknown ? "这次更改结果尚未确认，请明确核对当前记录；不会自动重发。" : asRefusal(problem).explanation); }
    }).finally(() => {
      if (slot !== null) assistanceWrites.finish(slot, unknown);
      if (owner.finish(ticket)) setBusy(false);
    });
  };
  return { value, busy, error, notice, run, blocked: busy || writes.busy || writes.unknown };
}
