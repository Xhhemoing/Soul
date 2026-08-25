/**
 * The export half of WP-B07: state → `.beadproj` text, and the download itself.
 *
 * D-IE-14: the enumeration is driven by the `projects` array, not by a "give me
 * every pattern document" repository method. Orphan documents (R-UP-2) are
 * therefore deliberately absent from an archive, and the repository surface
 * stays exactly the three methods `bead-v1` shipped. Exporting writes nothing.
 */

import {
  BEADPROJ_MIME,
  buildBeadprojEntry,
  buildBeadprojFile,
  type BeadprojEntry,
  type BeadprojFile,
} from "../../schema/beadproj.ts";
import type { ProjectId } from "../../stores/ids.ts";
import type {
  InventoryEntry,
  PatternDoc,
  ProgressCursor,
  Project,
} from "../../stores/types.ts";

export interface ArchiveSource {
  readonly projects: readonly Project[];
  readonly inventory: readonly InventoryEntry[];
  readonly progress: readonly ProgressCursor[];
  readonly loadPatternDoc: (id: ProjectId) => Promise<PatternDoc | null>;
}

export interface CollectedArchive {
  readonly file: BeadprojFile;
  /**
   * Null-source projects whose document could not be read. They are left out
   * rather than exported as an entry this app's own validator would reject.
   */
  readonly skippedProjects: number;
}

function cursorFor(
  progress: readonly ProgressCursor[],
  id: ProjectId,
): ProgressCursor | null {
  return progress.find((cursor) => cursor.projectId === id) ?? null;
}

export async function collectBeadprojArchive(source: ArchiveSource): Promise<CollectedArchive> {
  const entries: BeadprojEntry[] = [];
  let skippedProjects = 0;

  for (const project of source.projects) {
    const cursor = cursorFor(source.progress, project.id);
    if (project.sourcePatternId !== null) {
      // A gallery project's grid is a build-time fixture; its status, backdrop
      // and cursor are still the user's and still worth archiving (D-IE-14).
      entries.push(buildBeadprojEntry(project, null, cursor));
      continue;
    }
    const doc = await source.loadPatternDoc(project.id).catch(() => null);
    if (doc === null) {
      skippedProjects += 1;
      continue;
    }
    entries.push(buildBeadprojEntry(project, doc, cursor));
  }

  return { file: buildBeadprojFile(entries, source.inventory), skippedProjects };
}

/** A single project is an archive of length one — same schema, same reader. */
export function buildSingleProjectFile(
  project: Project,
  doc: PatternDoc,
  cursor: ProgressCursor | null,
): BeadprojFile {
  return buildBeadprojFile([buildBeadprojEntry(project, doc, cursor)]);
}

/**
 * B05 review MED-1, promoted to a rule for this work package (D-IE-16): the
 * object URL is minted inside the click handler and handed back once the
 * download has started. A URL minted in `useMemo` is minted on renders nobody
 * asked to download on, and leaks one per render.
 *
 * The handback is deferred by a task so the browser has taken the blob before
 * it is revoked; revoking in the same tick as `click()` can cancel the
 * download outright.
 */
export function downloadBeadproj(fileName: string, content: string): boolean {
  const factory = globalThis.URL?.createObjectURL;
  if (typeof factory !== "function") return false;

  const href = factory.call(globalThis.URL, new Blob([content], { type: BEADPROJ_MIME }));
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = fileName;
  anchor.click();
  setTimeout(() => {
    globalThis.URL?.revokeObjectURL?.(href);
  }, 0);
  return true;
}
