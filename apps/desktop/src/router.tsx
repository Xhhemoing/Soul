/**
 * A hash router, hand-rolled because the shell needs about forty lines of one.
 *
 * Most of WP09's routes are deliberately empty: the views behind them are
 * other work packages, and an empty route that says whose it is beats a
 * placeholder screen that looks like a feature.
 */

import { useEffect, useState } from "react";

export type RouteId =
  | "home"
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
  {
    id: "profile",
    path: "/profile",
    title: "灵魂档案",
    ownedBy: "WP09 功能视图",
    pending: "特质轴、语气与纠正锁定已经在核心里落地，这个视图还没有接上去。",
  },
  {
    id: "graph",
    path: "/graph",
    title: "人脉图",
    ownedBy: "WP09 功能视图",
    pending: "节点与边由核心从证据推导，这个视图还没有接上去。",
  },
  {
    id: "memory",
    path: "/memory",
    title: "自传记忆",
    ownedBy: "WP09 功能视图",
    pending: "记忆的增删改与遗忘影响面预览还没有接上去。",
  },
  {
    id: "draft",
    path: "/draft",
    title: "起草",
    ownedBy: "WP10",
    pending: "起草只写不发，本工作包不实现。这里现在没有输入框，也没有发送按钮。",
  },
  {
    id: "files",
    path: "/files",
    title: "文件计划",
    ownedBy: "WP11",
    pending:
      "v0.1 只做授权目录的只读扫描与计划预览。这里不会出现「执行」按钮：文件写入是 v0.1.1 的事。",
  },
  {
    id: "research",
    path: "/research",
    title: "研究预览",
    ownedBy: "WP09 功能视图",
    pending: "预览只在屏幕上出现，不落盘；这个视图还没有接上去。",
  },
  {
    id: "audit",
    path: "/audit",
    title: "审计",
    ownedBy: "WP09 功能视图",
    pending: "审计链只记事实不记正文，这个视图还没有接上去。",
  },
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
