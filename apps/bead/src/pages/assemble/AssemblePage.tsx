import { useEffect } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { occupiedCount } from "../../algo/grid.ts";
import { EmptyState } from "../../components/EmptyState.tsx";
import { PersistenceBanner } from "../../components/PersistenceBanner.tsx";
import { fixtureGridFor } from "../../fixtures/grids.ts";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { asProjectId, isProjectId } from "../../stores/ids.ts";
import { boardSourceOf, type BoardSource } from "../../stores/patterns.ts";
import { selectProject } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import { usePatternDoc } from "../../stores/usePatternDoc.ts";
import { AssembleBackdrop, BackdropControls } from "./AssembleBackdrop.tsx";
import { AssembleSession } from "./AssembleSession.tsx";
import { backdropColorOf, readableTextColor } from "./backdrop.ts";

export const NO_GRID_NOTE =
  "这个项目还没有豆图网格——空白项目的编辑器归 WP-B06，立体拼豆的多板拼接不在 v0。";
export const EMPTY_GRID_NOTE = "这张图纸的网格是空的，没有可拼的格子。";
export const LOADING_GRID_NOTE = "正在读取豆图……";

/**
 * D-UI-2, half two: this route renders outside AppShell — no bottom nav, no app
 * header. It is a page and not a modal, so there is no focus trap; the exit
 * control is always visible and Escape does the same thing.
 *
 * D-ASM-12 is the frame invariant: every project that can be found renders the
 * title, the way out, the backdrop controls and the persistence banner. Only
 * the session — canvas, HUD, control orb, timer — waits on there being a
 * fixture grid to assemble.
 */
export function AssemblePage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { projects, progress, hydrated, setProjectBackdrop } = useStore();
  const project = isProjectId(id) ? selectProject(projects, id) : undefined;
  useDocumentTitle(project ? `拼装 ${project.title}` : "拼装");

  // D-UP-14: two ways a grid gets here. A gallery project resolves its fixture
  // synchronously, exactly as before; a converted upload (`sourcePatternId`
  // null) reads its `PatternDoc` out of the `patterns` store. The hook runs for
  // both so the call order stays fixed, and asks for nothing when there is no
  // conversion to load.
  const conversionId =
    project !== undefined && project.sourcePatternId === null ? asProjectId(project.id) : null;
  const patternDoc = usePatternDoc(conversionId);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void navigate("/workspace");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [navigate]);

  if (!project) {
    // A guard screen, not a redirect: bouncing away would eat the back button.
    return (
      <div className="assemble">
        <div className="assemble__content">
          <h1>无法进入拼装</h1>
          <EmptyState
            message={
              hydrated
                ? "这个项目不存在。拼装只接受项目 id（proj- 开头）；画廊图纸要先「加入待拼」或「转入工作台」。"
                : "正在读取本地项目……"
            }
            actions={[{ label: "退出到拼装台", to: "/workspace" }]}
          />
        </div>
      </div>
    );
  }

  const backdropColor = backdropColorOf(project);
  const textColor = readableTextColor(backdropColor);
  const projectId = asProjectId(project.id);
  const cursor = progress.find((entry) => entry.projectId === projectId);

  const loadingBoard = conversionId !== null && patternDoc.status === "loading";
  let board: BoardSource | null = null;
  if (project.sourcePatternId !== null) board = fixtureGridFor(project.sourcePatternId);
  else if (patternDoc.status === "ready") board = boardSourceOf(patternDoc.doc);

  const note = loadingBoard
    ? LOADING_GRID_NOTE
    : board === null
      ? NO_GRID_NOTE
      : occupiedCount(board.grid) === 0
        ? EMPTY_GRID_NOTE
        : null;

  return (
    <div className="assemble" style={{ color: textColor }}>
      <AssembleBackdrop color={backdropColor} kind={project.backdrop} />
      <div className="assemble__content">
        <PersistenceBanner />
        <div className="assemble__bar">
          <h1>{project.title}</h1>
          <Link className="button" to="/workspace">
            退出拼装
          </Link>
        </div>

        <BackdropControls
          kind={project.backdrop}
          color={project.backdropColor}
          onSelect={(kind) => setProjectBackdrop(projectId, kind)}
          onCustomColor={(color) => setProjectBackdrop(projectId, "custom", color)}
        />

        {note !== null || board === null ? (
          <p className="assemble__note" aria-busy={loadingBoard || undefined}>
            {note}
          </p>
        ) : (
          <AssembleSession
            project={project}
            board={board}
            cursor={cursor}
            outlineColor={textColor}
          />
        )}
      </div>
    </div>
  );
}
