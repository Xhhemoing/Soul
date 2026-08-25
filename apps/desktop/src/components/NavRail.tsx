/** The left-hand rail. Links only; every route decides its own contents. */

import { ROUTES, type RouteDefinition } from "../router";

export interface NavRailProps {
  readonly current: RouteDefinition;
}

export function NavRail({ current }: NavRailProps): React.JSX.Element {
  return (
    <nav className="rail" aria-label="主导航">
      <p className="wordmark">Soul</p>
      <ul>
        {ROUTES.map((route) => (
          <li key={route.id}>
            <a
              href={`#${route.path}`}
              aria-current={route.id === current.id ? "page" : undefined}
              className={route.ownedBy === null ? "" : "muted"}
            >
              {route.title}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  );
}
