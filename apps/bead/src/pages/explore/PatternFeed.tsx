import { Link } from "react-router";

import type { Pattern } from "../../stores/types.ts";

/**
 * The whole card is one link and holds no second click target; a Fork button
 * arrives with WP-B08 and will sit outside the card.
 */
export function PatternFeed({ patterns, variant = "feed" }: { patterns: Pattern[]; variant?: "feed" | "grid" }) {
  return (
    <ul className={`pattern-feed pattern-feed--${variant}`}>
      {patterns.map((pattern) => (
        <li key={pattern.id}>
          <Link className="card pattern-card" to={`/pattern/${pattern.id}`}>
            <span className="pattern-card__preview" aria-hidden="true" />
            <strong>{pattern.title}</strong>
            <div className="stub-note">
              难度 {pattern.difficulty} · {pattern.beadCount} 颗 · {pattern.tags.join(" / ")}
            </div>
          </Link>
        </li>
      ))}
    </ul>
  );
}
