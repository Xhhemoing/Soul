const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
export function assistanceTarget(hash: string): string | null | "invalid" {
  if (hash === "#/assistance" || hash === "") return null;
  if (!hash.startsWith("#/assistance?") || hash.length > 128) return "invalid";
  const params = new URLSearchParams(hash.slice("#/assistance?".length));
  const ids = params.getAll("item");
  return ids.length === 1 && UUID.test(ids[0] ?? "") && [...params.keys()].every((key) => key === "item") ? ids[0]!.toLowerCase() : "invalid";
}
export function assistanceHash(itemId: string): string { return `#/assistance?item=${encodeURIComponent(itemId)}`; }
