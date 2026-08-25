import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { BackHeader } from "../../components/BackHeader.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { findCreator, findPattern } from "../../stores/catalog.ts";
import { isPatternId } from "../../stores/ids.ts";
import { useStore } from "../../stores/store.tsx";
import type { BoardKind } from "../../stores/types.ts";

const BOARD_LABEL: Record<BoardKind, string> = {
  "square-28": "方板 28×28",
  "square-56": "方板 56×56",
  hex: "六角板",
  round: "圆板",
};

export function PatternDetailPage() {
  const { id = "" } = useParams();
  const pattern = isPatternId(id) ? findPattern(id) : undefined;
  useDocumentTitle(pattern ? pattern.title : "图纸");

  const navigate = useNavigate();
  const { favorites, toggleFavorite, instantiatePattern } = useStore();
  const [queuedTitle, setQueuedTitle] = useState<string | null>(null);

  if (!pattern) {
    return (
      <>
        <BackHeader to="/explore" label="返回灵感" title="图纸" />
        <EmptyState message="图纸不存在或已下架" actions={[{ label: "返回灵感", to: "/explore" }]} />
      </>
    );
  }

  const creator = findCreator(pattern.creatorId);
  const favorited = favorites.includes(pattern.id);

  // The minimal instantiation the IA review insisted on: without it Workspace
  // stays empty forever and `/assemble/:id` is unreachable, which would make
  // the navigation tests self-congratulatory. Progress tracking is WP-B04.
  const addToTodo = () => {
    instantiatePattern(pattern, "todo");
    setQueuedTitle(pattern.title);
  };

  const openInWorkspace = () => {
    instantiatePattern(pattern, "active");
    void navigate("/workspace");
  };

  return (
    <>
      <BackHeader to="/explore" label="返回灵感" title={pattern.title} />

      <div className="pattern-card__preview" aria-hidden="true" />

      {creator && (
        <p>
          创作者：<Link to={`/creator/${creator.id}`}>{creator.name}</Link>
        </p>
      )}

      <dl className="meta-list">
        <div>
          <dt>难度</dt>
          <dd>{pattern.difficulty} / 5</dd>
        </div>
        <div>
          <dt>颗粒总数</dt>
          <dd>{pattern.beadCount}</dd>
        </div>
        <div>
          <dt>预估耗时</dt>
          <dd>{pattern.estimatedMinutes} 分钟</dd>
        </div>
        <div>
          <dt>板型</dt>
          <dd>{BOARD_LABEL[pattern.board]}</dd>
        </div>
      </dl>

      <section className="section" aria-label="配色清单">
        <h2 className="section__title">配色清单</h2>
        <ul className="palette-list">
          {pattern.palette.map((entry) => (
            <li key={entry.code}>
              <ColorSwatch code={entry.code} hex={entry.hex} name={entry.name} beads={entry.beads} />
            </li>
          ))}
        </ul>
      </section>

      <div className="detail-actions">
        <button type="button" className="button" onClick={addToTodo}>
          加入待拼
        </button>
        <button type="button" className="button button--primary" onClick={openInWorkspace}>
          转入工作台
        </button>
        <button
          type="button"
          className="button"
          aria-pressed={favorited}
          onClick={() => toggleFavorite(pattern.id)}
        >
          {favorited ? "已收藏" : "收藏"}
        </button>
      </div>

      {queuedTitle !== null && (
        <p role="status">
          已加入待拼：{queuedTitle}。<Link to="/workspace">去拼装台</Link>
        </p>
      )}
    </>
  );
}
