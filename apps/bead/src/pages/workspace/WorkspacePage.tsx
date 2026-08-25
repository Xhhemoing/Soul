import { Link } from "react-router";

import { Card } from "../../components/Card.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { ProgressBar } from "../../components/ProgressBar.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { selectDrafts, selectFinished, selectInProgress } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import type { Project } from "../../stores/types.ts";

/**
 * D-ED-17 ②: a project with no gallery pattern behind it owns its own grid, so
 * it can be reopened in the editor. This is the whole of WP-B06's change to
 * this page — and it is what rescues the draft card, which until now was a
 * title and no way forward.
 */
function EditLink({ project }: { project: Project }) {
  if (project.sourcePatternId !== null) return null;
  return (
    <Link className="button" to={`/edit/${project.id}`}>
      编辑豆图
    </Link>
  );
}

function ProjectCard({ project }: { project: Project }) {
  return (
    <Card>
      <strong>{project.title}</strong>
      <ProgressBar value={0} label={`${project.title} 进度`} />
      <Link className="button button--primary" to={`/assemble/${project.id}`}>
        继续拼豆
      </Link>
      <EditLink project={project} />
    </Card>
  );
}

export function WorkspacePage() {
  useDocumentTitle("拼装台");
  const { projects } = useStore();
  const inProgress = selectInProgress(projects);
  const drafts = selectDrafts(projects);
  const finished = selectFinished(projects);

  return (
    <>
      <h1>拼装台</h1>

      <section className="section" aria-label="正在拼">
        <h2 className="section__title">正在拼</h2>
        {inProgress.length > 0 ? (
          <ul className="project-list">
            {inProgress.map((project) => (
              <li key={project.id}>
                <ProjectCard project={project} />
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState
            message="还没有正在拼的项目。从灵感里挑一张图纸转入工作台，或者自己起一个新项目。"
            actions={[
              { label: "去灵感挑图纸", to: "/explore" },
              { label: "新建项目", to: "/create" },
            ]}
          />
        )}
      </section>

      <section className="section" aria-label="草稿与设计">
        <h2 className="section__title">草稿与设计</h2>
        {drafts.length > 0 ? (
          <ul className="project-list">
            {drafts.map((project) => (
              <li key={project.id}>
                <Card>
                  <strong>{project.title}</strong>
                  <EditLink project={project} />
                </Card>
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState
            message="草稿是还没开拼的自定义图纸，编辑到一半的设计会留在这里。"
            actions={[{ label: "新建草稿", to: "/create" }]}
          />
        )}
      </section>

      <section className="section" aria-label="历史完工">
        <h2 className="section__title">历史完工</h2>
        {finished.length > 0 ? (
          <ul className="project-list">
            {finished.map((project) => (
              <li key={project.id}>
                <Card>
                  <strong>{project.title}</strong>
                  <EditLink project={project} />
                </Card>
              </li>
            ))}
          </ul>
        ) : (
          <EmptyState message="拼完的项目会连同用时一起收在这里。" />
        )}
      </section>
    </>
  );
}
