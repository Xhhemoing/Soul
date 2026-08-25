import { useEffect } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { asProjectId, isProjectId } from "../../stores/ids.ts";
import { selectProject } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import type { BackdropKind } from "../../stores/types.ts";
import { BACKDROP_LABEL, backdropColorOf, readableTextColor } from "./backdrop.ts";

const BACKDROP_OPTIONS: BackdropKind[] = ["black", "white", "custom"];
const PLACEHOLDER_CELLS = 14 * 10;

/**
 * D-UI-2, half two: this route renders outside AppShell — no bottom nav, no app
 * header. It is a page and not a modal, so there is no focus trap; the exit
 * control is always visible and Escape does the same thing.
 *
 * The four step modes, the floating control ball and the timer are WP-B04. All
 * that lives here is the backdrop, a static grid and the way out.
 */
export function AssemblePage() {
  const { id = "" } = useParams();
  const navigate = useNavigate();
  const { projects, hydrated, setProjectBackdrop } = useStore();
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

  return (
    <div className="assemble" style={{ color: textColor }}>
      <div
        className="assemble__backdrop"
        data-testid="assemble-backdrop"
        data-backdrop={project.backdrop}
        style={{ background: backdropColor }}
      />
      <div className="assemble__content">
        <div className="assemble__bar">
          <h1>{project.title}</h1>
          <Link className="button" to="/workspace">
            退出拼装
          </Link>
        </div>

        <fieldset style={{ border: "none", padding: 0 }}>
          <legend>拼装背景（跟随项目，与应用主题无关）</legend>
          {BACKDROP_OPTIONS.map((kind) => (
            <label key={kind} style={{ marginRight: 12 }}>
              <input
                type="radio"
                name="backdrop"
                value={kind}
                checked={project.backdrop === kind}
                onChange={() => setProjectBackdrop(projectId, kind)}
              />
              {BACKDROP_LABEL[kind]}
            </label>
          ))}
          {project.backdrop === "custom" && (
            <label>
              背景色
              <input
                type="color"
                value={project.backdropColor}
                onChange={(event) => setProjectBackdrop(projectId, "custom", event.target.value)}
              />
            </label>
          )}
        </fieldset>

        <div className="assemble__canvas" role="img" aria-label="拼装网格占位">
          {Array.from({ length: PLACEHOLDER_CELLS }, (_, index) => (
            <span key={index} className="assemble__cell" />
          ))}
        </div>

        <p className="stub-note" style={{ color: textColor }}>
          四种步骤模式、悬浮控制球、计时与里程碑归 WP-B04；这里只有静态网格占位。
        </p>
      </div>
    </div>
  );
}
