import { useEffect, useMemo, useState } from "react";

import { asProjectId, type ProjectId } from "./ids.ts";
import { selectInProgress } from "./projects.ts";
import { useStore } from "./store.tsx";
import type { PatternDoc, Project } from "./types.ts";

/**
 * Reading a grid is the one thing in this app that cannot be synchronous: the
 * `patterns` store is IndexedDB and the document is thousands of cells. So the
 * loading window is a state callers render rather than a gap they paper over
 * (D-UP-14, and §4.3 for the shortage list).
 *
 * Both hooks keep the *answer* in state and derive the status from whether that
 * answer still matches what was asked. Storing the status itself would mean
 * writing "loading" back on every id change, which is a render the question
 * already implies.
 */
export type PatternDocState =
  | { readonly status: "loading" }
  | { readonly status: "ready"; readonly doc: PatternDoc }
  | { readonly status: "missing" };

const LOADING: PatternDocState = { status: "loading" };
const MISSING: PatternDocState = { status: "missing" };

interface Answer {
  readonly id: ProjectId;
  readonly doc: PatternDoc | null;
}

export function usePatternDoc(projectId: ProjectId | null): PatternDocState {
  const { loadPatternDoc } = useStore();
  const [answer, setAnswer] = useState<Answer | null>(null);

  useEffect(() => {
    if (projectId === null) return undefined;
    let cancelled = false;
    void (async () => {
      const doc = await loadPatternDoc(projectId).catch(() => null);
      if (!cancelled) setAnswer({ id: projectId, doc });
    })();
    return () => {
      cancelled = true;
    };
  }, [loadPatternDoc, projectId]);

  if (projectId === null) return MISSING;
  if (answer === null || answer.id !== projectId) return LOADING;
  return answer.doc === null ? MISSING : { status: "ready", doc: answer.doc };
}

export interface ConversionDocs {
  readonly docs: readonly PatternDoc[];
  readonly loading: boolean;
}

const NO_DOCS: readonly PatternDoc[] = [];

/**
 * §4.3: every `todo`/`active` project with no source pattern, resolved to its
 * stored document. Keyed on the ids alone, so re-rendering `/inventory` for an
 * unrelated reason does not re-read a single grid.
 */
export function useConversionDocs(projects: readonly Project[]): ConversionDocs {
  const idKey = useMemo(
    () =>
      selectInProgress(projects)
        .filter((project) => project.sourcePatternId === null)
        .map((project) => project.id)
        .join("\u0000"),
    [projects],
  );
  // Split back out of the joined key so the array identity is stable for as
  // long as the ids are: the effect below depends on it.
  const ids = useMemo(() => (idKey === "" ? [] : idKey.split("\u0000").map(asProjectId)), [idKey]);

  const { loadPatternDoc } = useStore();
  const [answer, setAnswer] = useState<{ key: string; docs: PatternDoc[] } | null>(null);

  useEffect(() => {
    if (ids.length === 0) return undefined;
    let cancelled = false;
    void (async () => {
      const loaded = await Promise.all(ids.map((id) => loadPatternDoc(id).catch(() => null)));
      if (cancelled) return;
      setAnswer({ key: idKey, docs: loaded.filter((doc): doc is PatternDoc => doc !== null) });
    })();
    return () => {
      cancelled = true;
    };
  }, [idKey, ids, loadPatternDoc]);

  return useMemo(() => {
    if (ids.length === 0) return { docs: NO_DOCS, loading: false };
    if (answer === null || answer.key !== idKey) return { docs: NO_DOCS, loading: true };
    return { docs: answer.docs, loading: false };
  }, [answer, idKey, ids]);
}
