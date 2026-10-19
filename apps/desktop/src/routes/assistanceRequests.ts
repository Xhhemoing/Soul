import { useSyncExternalStore } from "react";

interface WriteState { readonly busy: boolean; readonly unknown: boolean }
let state: WriteState = { busy: false, unknown: false };
let active: symbol | null = null;
let activeRecord: string | null = null;
let unknownRecord: string | null = null;
let unknownToken: symbol | null = null;
const listeners = new Set<() => void>();
function publish(next: WriteState): void { state = next; for (const listener of [...listeners]) listener(); }
/** Cross-mount physical write admission. Unknown outcomes are read back, never replayed. */
export const assistanceWrites = {
  snapshot: (): WriteState => state,
  subscribe: (listener: () => void): (() => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
  begin: (record: string | null = null): symbol | null => {
    if (active !== null || state.unknown) return null;
    activeRecord = record; active = Symbol("assistance write"); publish({ ...state, busy: true }); return active;
  },
  finish: (slot: symbol, unknown: boolean): void => {
    if (slot !== active) return;
    if (unknown) { unknownRecord = activeRecord; unknownToken = Symbol("unknown assistance write"); }
    active = null; activeRecord = null; publish({ busy: false, unknown: state.unknown || unknown });
  },
  recovery: (record: string): symbol | null => active === null && state.unknown && record === unknownRecord ? unknownToken : null,
  checked: (record?: string, token?: symbol | null): void => {
    if (active === null && (record === undefined ? unknownRecord === null : record === unknownRecord && token !== null && token === unknownToken)) {
      unknownRecord = null; unknownToken = null; publish({ busy: false, unknown: false });
    }
  },
};
export function useAssistanceWrites(): WriteState { return useSyncExternalStore(assistanceWrites.subscribe, assistanceWrites.snapshot); }
