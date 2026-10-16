import { requestDraftDiscard } from "./draftDiscard";
let candidates: readonly string[] = [];
const listeners = new Set<() => void>();
export const draftMemoryCandidates = {
  snapshot: (): readonly string[] => candidates,
  subscribe: (listener: () => void): (() => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; },
  add: (id: string): void => {
    if (candidates.includes(id) || candidates.length >= 32) return;
    candidates = [...candidates, id]; requestDraftDiscard(); listeners.forEach((listener) => listener());
  },
  remove: (id: string): void => {
    candidates = candidates.filter((value) => value !== id); requestDraftDiscard(); listeners.forEach((listener) => listener());
  },
};
