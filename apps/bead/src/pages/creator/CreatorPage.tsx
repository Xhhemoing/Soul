import { useParams } from "react-router";

import { BackHeader } from "../../components/BackHeader.tsx";
import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { findCreator, patternsByCreator } from "../../stores/catalog.ts";
import { isCreatorId } from "../../stores/ids.ts";
import { PatternFeed } from "../explore/PatternFeed.tsx";

export function CreatorPage() {
  const { id = "" } = useParams();
  const creator = isCreatorId(id) ? findCreator(id) : undefined;
  useDocumentTitle(creator ? creator.name : "创作者");

  if (!creator) {
    return (
      <>
        <BackHeader to="/explore" label="返回灵感" title="创作者" />
        <EmptyState message="创作者不存在" actions={[{ label: "返回灵感", to: "/explore" }]} />
      </>
    );
  }

  const patterns = patternsByCreator(creator.id);

  return (
    <>
      <BackHeader to="/explore" label="返回灵感" title={creator.name} />
      <p>{creator.bio}</p>
      <p>被拼打卡数：{creator.checkIns}</p>
      <section className="section" aria-label="作品集">
        <h2 className="section__title">作品集</h2>
        {patterns.length > 0 ? (
          <PatternFeed patterns={patterns} variant="grid" />
        ) : (
          <EmptyState message="这位创作者还没有公开图纸。" actions={[{ label: "返回灵感", to: "/explore" }]} />
        )}
        <p className="stub-note">Fork 二创改色归 WP-B08，B02 不放死按钮。</p>
      </section>
    </>
  );
}
