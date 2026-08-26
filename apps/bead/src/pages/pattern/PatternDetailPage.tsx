import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router";

import { BackHeader } from "../../components/BackHeader.tsx";
import { ColorSwatch } from "../../components/ColorSwatch.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { fixtureGridFor } from "../../fixtures/grids.ts";
import { findCreator, findPattern } from "../../stores/catalog.ts";
import { isPatternId, mintProjectId } from "../../stores/ids.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import { createProjectFromFork } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import type { BoardKind } from "../../stores/types.ts";
import { forkFixture, forkTitle } from "./fork.ts";
import { PatternPreview } from "./PatternPreview.tsx";

const BOARD_LABEL: Record<BoardKind, string> = {
  "square-28": "方板 28×28",
  "square-56": "方板 56×56",
  hex: "六角板",
  round: "圆板",
};

/** D-GAL-10: disabled with the reason on screen, never hidden. */
export const FORK_UNAVAILABLE_NOTE = "这张图纸还没有网格，暂不能 Fork";
export const FORK_SAVE_ERROR = "Fork 没有保存，项目也没有创建";

export function PatternDetailPage() {
  const { id = "" } = useParams();
  const pattern = isPatternId(id) ? findPattern(id) : undefined;
  useDocumentTitle(pattern ? pattern.title : "图纸");

  const navigate = useNavigate();
  const { favorites, toggleFavorite, instantiatePattern, addProject, savePatternDoc } = useStore();
  const [queuedTitle, setQueuedTitle] = useState<string | null>(null);
  const [forkError, setForkError] = useState<string | null>(null);
  const [forking, setForking] = useState(false);

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
  // Synchronous: the grids are build-time code, so a fork has no loading state.
  const fixture = fixtureGridFor(pattern.id);

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

  /**
   * D-GAL-7, which is D-UP-10's order reused a third time: the quantised
   * document is awaited onto disk, and only a document that landed gets a
   * project pointing at it. A failed write mints nothing, navigates nowhere and
   * says so inline (D-UP-16) — the alternative is a project that opens to an
   * empty board and no explanation.
   */
  async function fork(): Promise<void> {
    if (pattern === undefined || fixture === null || forking) return;
    setForking(true);
    setForkError(null);

    const projectId = mintProjectId();
    // No provenance: a fork is neither a PixelArt nor a Photo conversion, and
    // widening that union for one display-only field is D-GAL-9's judgement.
    const doc = createPatternDoc(projectId, forkFixture(fixture).grid);

    try {
      await savePatternDoc(doc);
    } catch {
      setForkError(FORK_SAVE_ERROR);
      setForking(false);
      return;
    }

    addProject(createProjectFromFork(projectId, forkTitle(pattern.title)));
    setForking(false);
    void navigate(`/edit/${projectId}`);
  }

  return (
    <>
      <BackHeader to="/explore" label="返回灵感" title={pattern.title} />

      {fixture === null ? (
        <div className="pattern-card__preview" aria-hidden="true" />
      ) : (
        <PatternPreview fixture={fixture} />
      )}

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
        {/* D-GAL-11: the fork lives here rather than on the card — a PatternCard
            is one link end to end (round1 §6) and has no room for the reason a
            gridless pattern cannot be forked. */}
        <button
          type="button"
          className="button"
          disabled={fixture === null || forking}
          onClick={() => void fork()}
        >
          Fork 改色
        </button>
      </div>

      {fixture === null && <p className="stub-note">{FORK_UNAVAILABLE_NOTE}</p>}

      {forkError !== null && (
        <p className="stock-form__error" role="alert">
          {forkError}
        </p>
      )}

      {queuedTitle !== null && (
        <p role="status">
          已加入待拼：{queuedTitle}。<Link to="/workspace">去拼装台</Link>
        </p>
      )}
    </>
  );
}
