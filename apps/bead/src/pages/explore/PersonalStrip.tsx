import { Link } from "react-router";

import { Card } from "../../components/Card.tsx";
import { ProgressBar } from "../../components/ProgressBar.tsx";
import { selectActiveProject, selectTodoCount } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";

/**
 * Three cells that always occupy the same space. Counts render as `0` rather
 * than disappearing so the strip does not reflow the moment a project lands.
 */
export function PersonalStrip() {
  const { projects, favorites } = useStore();
  const active = selectActiveProject(projects);
  const todoCount = selectTodoCount(projects);

  return (
    <section className="personal-strip" aria-label="个人条">
      <Card>
        {active ? (
          <Link className="chip-link" to={`/assemble/${active.id}`}>
            <strong>{active.title}</strong>
            <ProgressBar value={0} label={`${active.title} 进度`} />
            <span className="stub-note">进度与用时统计归 WP-B04</span>
          </Link>
        ) : (
          <>
            <p className="empty-state__message">还没有正在拼的项目</p>
            <Link className="button button--primary" to="/create">
              开始一个项目
            </Link>
          </>
        )}
      </Card>
      <Card>
        <Link className="chip-link" to="/workspace">
          <span className="chip-link__count">{todoCount}</span>
          待办
        </Link>
      </Card>
      <Card>
        <Link className="chip-link" to="/explore?fav=1">
          <span className="chip-link__count">{favorites.length}</span>
          收藏
        </Link>
      </Card>
    </section>
  );
}
