const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** A navigation intent for one read, never an edit or forget authorization. */
export function memoryTarget(hash: string): string | null | "invalid" {
  if (hash === "#/memory" || hash === "") return null;
  if (!hash.startsWith("#/memory?") || hash.length > 128) return "invalid";
  const params = new URLSearchParams(hash.slice("#/memory?".length));
  const ids = params.getAll("memory_id");
  return ids.length === 1 && UUID.test(ids[0] ?? "") && [...params.keys()].every((key) => key === "memory_id")
    ? ids[0]!.toLowerCase() : "invalid";
}
export function memoryHash(memoryId: string): string { return `#/memory?memory_id=${encodeURIComponent(memoryId.toLowerCase())}`; }
