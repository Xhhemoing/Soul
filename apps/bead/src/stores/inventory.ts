import type { InventoryEntry } from "./types.ts";

// WP-B05 owns inventory CRUD, shortage warnings and ΔE00<3 substitutes. B02
// only needs enough to render the panels and their zero states.

export function selectStock(entries: readonly InventoryEntry[]): InventoryEntry[] {
  return [...entries].sort((a, b) => a.code.localeCompare(b.code));
}

export function totalBeads(entries: readonly InventoryEntry[]): number {
  return entries.reduce((sum, entry) => sum + entry.beads, 0);
}
