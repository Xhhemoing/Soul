import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { occupiedCount } from "../../algo/grid.ts";
import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { asProjectId, isProjectId } from "../../stores/ids.ts";
import { createPatternDoc, decodeCells } from "../../stores/patterns.ts";
import { selectProject } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import { usePatternDoc } from "../../stores/usePatternDoc.ts";
import type { PatternDoc, ProgressCursor, Project } from "../../stores/types.ts";
import { EditorCanvas } from "./EditorCanvas.tsx";
import { EditorTools } from "./EditorTools.tsx";
import {
  EDITOR_SWATCHES,
  createEditorState,
  editorBom,
  editorReduce,
  type Point,
} from "./editor.ts";

/**
 * WP-B06: `/edit/:id`, the pixel editor.
 *
 * D-ED-3 puts it on a route of its own rather than in a `/create` panel: the
 * project being edited is a state that can be linked, backed out of and
 * re-entered from the Workspace, which is exactly what D-UI-1 says belongs in
 * the URL. It renders inside AppShell (bottom nav, no highlighted tab) the way
 * `/pattern/:id` does — an editor is not the immersive chrome an assembly
 * session is.
 *
 * D-ED-1: the editable set is projects with no gallery pattern behind them and
 * a document that loads. A gallery fixture is build-time read-only code and its
 * palette is a per-pattern gallery table that `PatternDoc` cannot name, so
 * those get a guard screen pointing at WP-B08 rather than a broken canvas.
 */

export const HYDRATING_NOTE = "正在读取本地项目……";
export const UNKNOWN_PROJECT_NOTE =
  "这个项目不存在。编辑器只接受项目 id（proj- 开头）；画廊图纸要先「加入待拼」或「转入工作台」。";
export const GALLERY_NOTE =
  "画廊图纸的网格是只读的——Fork 改色归 WP-B08，现在还改不了它的色板。";
export const LOADING_DOC_NOTE = "正在读取豆图……";
export const NO_DOC_NOTE =
  "这个项目还没有豆图文档。它不是空白项目，也没有转换结果可以编辑。";
export const CURSOR_RESET_NOTE = "编辑会使拼装进度回到第 1 步";

/** D-ED-15: quiet enough to swallow a stroke, short enough to survive a tab close. */
export const AUTOSAVE_DELAY_MS = 800;

type SaveStatus = "idle" | "saving" | "saved" | "failed";

const SAVE_LABEL: Record<SaveStatus, string> = {
  idle: "",
  saving: "保存中…",
  saved: "已保存",
  failed: "保存失败",
};

/** D-ASM-13 precedent: a global shortcut yields to whatever is being typed into. */
function typesInto(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return ["INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);
}

/** D-ED-18: every guard screen states the reason and keeps a way out; none of them redirects. */
function EditGuard({ message, busy = false }: { message: string; busy?: boolean }) {
  return (
    <>
      <h1>豆图编辑器</h1>
      <div {...(busy ? { "aria-busy": true } : {})}>
        <EmptyState message={message} actions={[{ label: "回拼装台", to: "/workspace" }]} />
      </div>
    </>
  );
}

export function EditPage() {
  const { id = "" } = useParams();
  const { projects, progress, hydrated } = useStore();
  const project = isProjectId(id) ? selectProject(projects, id) : undefined;
  useDocumentTitle(project ? `编辑 ${project.title}` : "豆图编辑器");

  // Asked for only when the project is one this editor may open, so a gallery
  // project never triggers a read it would have to throw away.
  const editableId =
    project !== undefined && project.sourcePatternId === null ? asProjectId(project.id) : null;
  const docState = usePatternDoc(editableId);

  if (!hydrated) return <EditGuard message={HYDRATING_NOTE} busy />;
  if (project === undefined) return <EditGuard message={UNKNOWN_PROJECT_NOTE} />;
  if (project.sourcePatternId !== null) return <EditGuard message={GALLERY_NOTE} />;
  if (docState.status === "loading") return <EditGuard message={LOADING_DOC_NOTE} busy />;
  if (docState.status === "missing") return <EditGuard message={NO_DOC_NOTE} />;

  const cursor = progress.find((entry) => entry.projectId === project.id);
  return (
    <EditorSurface
      key={project.id}
      project={project}
      doc={docState.doc}
      {...(cursor === undefined ? {} : { cursor })}
    />
  );
}

interface EditorSurfaceProps {
  readonly project: Project;
  readonly doc: PatternDoc;
  readonly cursor?: ProgressCursor;
}

function EditorSurface({ project, doc, cursor }: EditorSurfaceProps) {
  const navigate = useNavigate();
  const { savePatternDoc, setProjectStatus, upsertProgress } = useStore();
  const projectId = asProjectId(project.id);

  const [state, dispatch] = useReducer(editorReduce, doc, (seed) =>
    createEditorState(decodeCells(seed)),
  );
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");

  // What the save and the pointer callbacks read. Keeping it in a ref is what
  // lets both of them stay identity-stable, which is what lets the memoised
  // canvas skip its 3136 nodes while the tool panel re-renders.
  const latest = useRef({ state, cursor });
  useEffect(() => {
    latest.current = { state, cursor };
  });

  const savedRevision = useRef(0);
  const provenance = doc.provenance;

  /**
   * D-ED-15: the whole document, upserted. The editor opened a document that
   * already exists, so there is no minting sequence here and no explicit save
   * key — just the last state that stopped changing 800ms ago.
   */
  const flush = useCallback(async (): Promise<void> => {
    const { state: current, cursor: currentCursor } = latest.current;
    if (current.revision === savedRevision.current) return;
    savedRevision.current = current.revision;
    setSaveStatus("saving");
    try {
      await savePatternDoc(createPatternDoc(projectId, current.grid, provenance));
    } catch {
      // The revision stays unclaimed so the next edit retries instead of
      // leaving the board stranded in memory (D-UP-16: never catch-and-pretend).
      savedRevision.current = -1;
      setSaveStatus("failed");
      return;
    }
    setSaveStatus("saved");
    // D-ED-16: the grid moved, so a stored step index now points at the wrong
    // bead — worse than losing it. Mode and elapsed time are the project's own
    // history and survive (the D-ASM-4 mirror).
    if (currentCursor !== undefined && currentCursor.stepIndex !== 0) {
      upsertProgress({
        projectId,
        mode: currentCursor.mode,
        stepIndex: 0,
        elapsedMs: currentCursor.elapsedMs,
      });
    }
  }, [projectId, provenance, savePatternDoc, upsertProgress]);

  const revision = state.revision;
  useEffect(() => {
    if (revision === 0) return undefined;
    const timer = setTimeout(() => void flush(), AUTOSAVE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [revision, flush]);

  // R-ED-5: the debounce window is the only place a stroke can be lost, and a
  // normal close is not that window — `pagehide` and unmount both flush.
  useEffect(() => {
    const onPageHide = () => void flush();
    window.addEventListener("pagehide", onPageHide);
    return () => {
      window.removeEventListener("pagehide", onPageHide);
      void flush();
    };
  }, [flush]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      if (typesInto(event.target)) return;
      const key = event.key.toLowerCase();
      if (key === "z") {
        event.preventDefault();
        dispatch(event.shiftKey ? { kind: "redo" } : { kind: "undo" });
      } else if (key === "y") {
        event.preventDefault();
        dispatch({ kind: "redo" });
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const onBegin = useCallback((point: Point) => {
    const { tool } = latest.current.state;
    if (tool === "brush") dispatch({ kind: "strokeBegin", point });
    else if (tool === "bucket") dispatch({ kind: "fill", point });
    else dispatch({ kind: "pick", point });
  }, []);

  const onExtend = useCallback((points: readonly Point[]) => {
    if (latest.current.state.tool !== "brush") return;
    dispatch({ kind: "strokeTo", points });
  }, []);

  const onEnd = useCallback(() => dispatch({ kind: "strokeEnd" }), []);

  const bom = useMemo(() => editorBom(state.grid), [state.grid]);
  const beads = occupiedCount(state.grid);

  // D-ED-17 ③: an empty board has nothing to assemble, so the door to the
  // session stays shut until something is on it. A draft is promoted on the way
  // through; anything already todo/active/done just navigates.
  async function goAssemble(): Promise<void> {
    await flush();
    if (project.status === "draft") setProjectStatus(projectId, "active");
    void navigate(`/assemble/${projectId}`);
  }

  return (
    <>
      <header className="section editor__header">
        <Link className="button" to="/workspace">
          ← 回拼装台
        </Link>
        <h1>{project.title}</h1>
        <p className="editor__save" role="status">
          {SAVE_LABEL[saveStatus]}
        </p>
        <button
          type="button"
          className="button button--primary"
          disabled={beads === 0}
          onClick={() => void goAssemble()}
        >
          去拼装
        </button>
      </header>

      {cursor !== undefined && <p className="stub-note">{CURSOR_RESET_NOTE}</p>}

      <div className="editor">
        <EditorCanvas
          grid={state.grid}
          palette={EDITOR_SWATCHES}
          label={`豆图画布：${state.grid.width}×${state.grid.height} 网格，共 ${beads} 颗豆`}
          onBegin={onBegin}
          onExtend={onExtend}
          onEnd={onEnd}
        />

        <EditorTools
          tool={state.tool}
          activeColor={state.activeColor}
          symmetry={state.symmetry}
          bom={bom}
          total={beads}
          canUndo={state.past.length > 0}
          canRedo={state.future.length > 0}
          onTool={(tool) => dispatch({ kind: "selectTool", tool })}
          onColor={(color) => dispatch({ kind: "selectColor", color })}
          onSymmetry={(symmetry) => dispatch({ kind: "selectSymmetry", symmetry })}
          onUndo={() => dispatch({ kind: "undo" })}
          onRedo={() => dispatch({ kind: "redo" })}
          onReplace={(from, to) => dispatch({ kind: "replace", from, to })}
        />
      </div>
    </>
  );
}
