/**
 * A hash router, hand-rolled because the shell needs about forty lines of one.
 *
 * `ownedBy` names the work package that still has to fill a route in, and it
 * is null on every route now that the last four have been. The field and the
 * `Pending` screen behind it are kept rather than deleted: an empty route that
 * says whose it is beats a placeholder that looks like a feature, and the next
 * route to be added before its view exists should get the same treatment
 * rather than a screen someone might wire a button to.
 */

import { useEffect, useState } from "react";

export type RouteId =
  | "home"
  | "import"
  | "collect"
  | "profile"
  | "graph"
  | "memory"
  | "draft"
  | "files"
  | "research"
  | "audit"
  | "settings";

export interface RouteDefinition {
  readonly id: RouteId;
  readonly path: string;
  readonly title: string;
  /** The work package that fills this route in, or null when it is filled. */
  readonly ownedBy: string | null;
  /** What is deliberately not here yet, in the user's words. */
  readonly pending?: string;
}

export const ROUTES: readonly RouteDefinition[] = [
  { id: "home", path: "/", title: "概览", ownedBy: null },
  { id: "import", path: "/import", title: "导入", ownedBy: null },
  { id: "collect", path: "/collect", title: "采集", ownedBy: null },
  { id: "profile", path: "/profile", title: "灵魂档案", ownedBy: null },
  { id: "graph", path: "/graph", title: "人脉图", ownedBy: null },
  { id: "memory", path: "/memory", title: "自传记忆", ownedBy: null },
  { id: "draft", path: "/draft", title: "起草", ownedBy: null },
  { id: "files", path: "/files", title: "文件计划", ownedBy: null },
  { id: "research", path: "/research", title: "研究预览", ownedBy: null },
  { id: "audit", path: "/audit", title: "审计", ownedBy: null },
  { id: "settings", path: "/settings", title: "设置", ownedBy: null },
];

const HOME = ROUTES[0] as RouteDefinition;

export function routeForHash(hash: string): RouteDefinition {
  const path = hash.replace(/^#/, "") || "/";
  return ROUTES.find((route) => route.path === path) ?? HOME;
}

/** The current route, kept in sync with the address the WebView is showing. */
export function useRoute(): RouteDefinition {
  const [route, setRoute] = useState(() => routeForHash(window.location.hash));

  useEffect(() => {
    const onHashChange = (): void => setRoute(routeForHash(window.location.hash));
    window.addEventListener("hashchange", onHashChange);
    return () => window.removeEventListener("hashchange", onHashChange);
  }, []);

  return route;
}
