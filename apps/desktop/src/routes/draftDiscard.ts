import { discardDraft } from "../core";
import { draftLane } from "./draftRequests";
let requested = false;

/** Input edits invalidate immediately; this drains a native preparation only
 * after its previous physical operation has settled, across route mounts. */
export function requestDraftDiscard(): void { requested = true; drain(); }
function drain(): void {
  if (!requested || draftLane.snapshot().busy || draftLane.snapshot().uncertain) return;
  const slot = draftLane.begin("discard");
  if (slot === null) return;
  requested = false;
  void Promise.resolve().then(() => discardDraft()).then(
    () => { draftLane.finish(slot, false); },
    () => { draftLane.finish(slot, true); },
  );
}
draftLane.subscribe(drain);
