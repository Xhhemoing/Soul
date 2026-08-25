import { useEffect } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { occupiedCount } from "../../algo/grid.ts";
import { EmptyState } from "../../components/EmptyState.tsx";
import { PersistenceBanner } from "../../components/PersistenceBanner.tsx";
import { fixtureGridFor } from "../../fixtures/grids.ts";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { asProjectId, isProjectId } from "../../stores/ids.ts";
import { selectProject } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import { AssembleBackdrop, BackdropControls } from "./AssembleBackdrop.tsx";
import { AssembleSession } from "./AssembleSession.tsx";
import { backdropColorOf, readableTextColor } from "./backdrop.ts";

export const NO_GRID_NOTE =
  "这个项目还没有豆图网格——上传转图归 WP-B03，立体拼豆的多板拼接不在 v0。";
export const EMPTY_GRID_NOTE = "这张图纸的网格是空的，没有可拼的格子。";

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
  const fixture = project.sourcePatternId === null ? null : fixtureGridFor(project.sourcePatternId);
  const cursor = progress.find((entry) => entry.projectId === projectId);

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

        {fixture === null || occupiedCount(fixture.grid) === 0 ? (
          <p className="assemble__note">
            {fixture === null ? NO_GRID_NOTE : EMPTY_GRID_NOTE}
          </p>
        ) : (
          <AssembleSession
            project={project}
            fixture={fixture}
            cursor={cursor}
            outlineColor={textColor}
          />
        )}
      </div>
    </div>
  );
}
