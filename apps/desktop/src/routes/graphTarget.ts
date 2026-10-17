import type { EvidenceRow, PeopleGraph, PersonNode, TieEdge } from "../core";

/** Local navigation budgets, unrelated to the frozen relationship algorithms. */
export const MAX_GRAPH_TARGET_REFERENCES = 64;
export const MAX_GRAPH_TARGET_HASH_CHARS = 8192;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export type GraphTarget =
  | { readonly kind: "none" }
  | { readonly kind: "invalid" }
  | { readonly kind: "selection"; readonly contactId: string; readonly evidenceIds: readonly string[] };
export type GraphTargetResolution =
  | { readonly kind: "none" }
  | { readonly kind: "unavailable" }
  | { readonly kind: "ready"; readonly contact: PersonNode; readonly tie: TieEdge; readonly evidence: readonly EvidenceRow[] };

export function parseGraphTarget(hash: string): GraphTarget {
  const path = hash.replace(/^#/, "");
  if (path === "/graph" || !path.startsWith("/graph?")) return { kind: "none" };
  if (hash.length > MAX_GRAPH_TARGET_HASH_CHARS) return { kind: "invalid" };
  const params = new URLSearchParams(path.slice("/graph?".length));
  if ([...params.keys()].some((key) => key !== "contact" && key !== "evidence")) return { kind: "invalid" };
  const contacts = params.getAll("contact"); const references = params.getAll("evidence");
  if (contacts.length !== 1 || !UUID.test(contacts[0] ?? "") || references.length === 0 || references.length > MAX_GRAPH_TARGET_REFERENCES
    || references.some((id) => !UUID.test(id))) return { kind: "invalid" };
  const evidenceIds = references.map((id) => id.toLowerCase());
  if (new Set(evidenceIds).size !== evidenceIds.length) return { kind: "invalid" };
  return { kind: "selection", contactId: contacts[0]!.toLowerCase(), evidenceIds };
}

/** Too many references remove the precise link as a whole, never truncate it. */
export function graphTargetHash(contactId: string, references: readonly string[]): string | null {
  if (!UUID.test(contactId) || references.some((id) => !UUID.test(id))) return null;
  const ids = [...new Set(references.map((id) => id.toLowerCase()))];
  if (ids.length === 0 || ids.length > MAX_GRAPH_TARGET_REFERENCES) return null;
  const params = new URLSearchParams({ contact: contactId.toLowerCase() });
  for (const id of ids) params.append("evidence", id);
  const hash = `#/graph?${params.toString()}`;
  return hash.length <= MAX_GRAPH_TARGET_HASH_CHARS ? hash : null;
}

/** Only a unique current self/contact edge containing every requested id is
 * eligible. Presence of references is not verification of an old statement. */
export function resolveGraphTarget(graph: PeopleGraph, target: GraphTarget): GraphTargetResolution {
  if (target.kind === "none") return { kind: "none" };
  if (target.kind === "invalid" || graph.self_contact_id === null) return { kind: "unavailable" };
  const self = graph.self_contact_id.toLowerCase();
  const contact = graph.people.find((person) => person.contact_id.toLowerCase() === target.contactId);
  const owner = graph.people.find((person) => person.contact_id.toLowerCase() === self);
  if (contact === undefined || contact.forgotten || contact.is_you || target.contactId === self || owner === undefined || owner.forgotten) return { kind: "unavailable" };
  const matches = graph.ties.filter((tie) => {
    const from = tie.from_contact_id.toLowerCase(); const to = tie.to_contact_id.toLowerCase();
    return ((from === self && to === target.contactId) || (to === self && from === target.contactId))
      && target.evidenceIds.every((id) => tie.evidence.some((row) => row.evidence_id.toLowerCase() === id));
  });
  if (matches.length !== 1) return { kind: "unavailable" };
  const tie = matches[0]!;
  return { kind: "ready", contact, tie, evidence: target.evidenceIds.map((id) => tie.evidence.find((row) => row.evidence_id.toLowerCase() === id)!) };
}
