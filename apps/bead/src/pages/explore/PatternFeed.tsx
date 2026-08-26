import { Link } from "react-router";

import type { Pattern } from "../../stores/types.ts";

/**
 * The whole card is one link and holds no second click target (round1 §6).
 * 「Fork 改色」is one tap further in, on the pattern detail page, where there is
 * room to say why a gridless pattern cannot be forked (D-GAL-11).
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
