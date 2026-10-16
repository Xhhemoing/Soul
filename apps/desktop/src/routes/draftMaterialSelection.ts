import { requestDraftDiscard } from "./draftDiscard";

export interface InsightSelection {
  readonly bundle_id: string;
  readonly contact_id: string;
  readonly revision: number;
}

let memory: readonly string[] = [];
let insight: InsightSelection | null = null;
let revision = 0;
const listeners = new Set<() => void>();
const subscribe = (listener: () => void): (() => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };
function changed(): void {
  revision += 1;
  // Clearing an in-memory choice is not evidence that the native preparation
  // has gone away. This only drains discard, never a material read or send.
  requestDraftDiscard();
  listeners.forEach((listener) => listener());
}

/** Window-local UUID intent, never a context grant or a cached summary. */
export const draftMaterialSelection = { subscribe, generation: (): number => revision };
export const draftMemoryCandidates = {
  snapshot: (): readonly string[] => memory,
  subscribe,
  add: (id: string): void => {
    if (memory.includes(id) || memory.length >= 32) return;
    memory = [...memory, id]; insight = null; changed();
  },
  remove: (id: string): void => {
    if (!memory.includes(id)) return;
    memory = memory.filter((value) => value !== id); changed();
  },
};
export const draftInsightSelection = {
  snapshot: (): InsightSelection | null => insight,
  subscribe,
  select: (bundleId: string, contactId: string): void => {
    memory = [];
    insight = { bundle_id: bundleId, contact_id: contactId, revision: revision + 1 };
    changed();
  },
  clear: (): void => { if (insight !== null) { insight = null; changed(); } },
};
