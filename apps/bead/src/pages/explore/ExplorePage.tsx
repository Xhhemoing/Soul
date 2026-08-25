import { Link, useSearchParams } from "react-router";

import { EmptyState } from "../../components/EmptyState.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";
import { allPatterns, catalogTags, filterPatterns } from "../../stores/catalog.ts";
import { useStore } from "../../stores/store.tsx";
import { PatternFeed } from "./PatternFeed.tsx";
import { PersonalStrip } from "./PersonalStrip.tsx";

// D-UI-1: the tag and favourites filters live in the URL, so a filtered feed
// is shareable and the back button undoes a filter.
export function ExplorePage() {
  useDocumentTitle("灵感");
  const [params] = useSearchParams();
  const { favorites } = useStore();
  const tag = params.get("tag");
  const favoritesOnly = params.get("fav") === "1";

  const catalog = allPatterns();
  const patterns = filterPatterns(catalog, { tag, favoritesOnly, favorites });
  const filtered = tag !== null || favoritesOnly;

  return (
    <>
      <h1>灵感</h1>
      <PersonalStrip />
      <section className="section" aria-label="发现流">
        <div className="tag-row">
          <Link className="button" to="/explore">
            全部
          </Link>
          {catalogTags.map((name) => (
            <Link
              key={name}
              className="button"
              to={`/explore?tag=${encodeURIComponent(name)}`}
              aria-current={tag === name ? "page" : undefined}
            >
              {name}
            </Link>
          ))}
        </div>
        {patterns.length > 0 ? (
          <PatternFeed patterns={patterns} />
        ) : filtered ? (
          <EmptyState message="没有匹配的图纸" actions={[{ label: "清除筛选", to: "/explore" }]} />
        ) : (
          // Defensive: the fixture ships five patterns, so an empty catalog
          // means something upstream failed rather than a filter being on.
          <EmptyState message="没有匹配的图纸" actions={[{ label: "去创作", to: "/create" }]} />
        )}
      </section>
    </>
  );
}
