import { useSyncExternalStore } from "react";

interface WriteState { readonly busy: boolean; readonly unknown: boolean }
let state: WriteState = { busy: false, unknown: false };
let active: symbol | null = null;
const listeners = new Set<() => void>();
function publish(next: WriteState): void { state = next; for (const listener of [...listeners]) listener(); }
/** Cross-mount physical write admission. Unknown outcomes are read back, never replayed. */
export const assistanceWrites = {
  snapshot: (): WriteState => state,
  subscribe: (listener: () => void): (() => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
  begin: (): symbol | null => {
    if (active !== null || state.unknown) return null;
    active = Symbol("assistance write"); publish({ ...state, busy: true }); return active;
  },
  finish: (slot: symbol, unknown: boolean): void => {
    if (slot !== active) return;
    active = null; publish({ busy: false, unknown: state.unknown || unknown });
  },
  checked: (): void => { if (active === null) publish({ busy: false, unknown: false }); },
};
export function useAssistanceWrites(): WriteState { return useSyncExternalStore(assistanceWrites.subscribe, assistanceWrites.snapshot); }
