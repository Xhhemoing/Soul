import { Outlet } from "react-router";

import { BottomNav } from "./BottomNav.tsx";
import { ThemeToggle } from "./ThemeToggle.tsx";

/**
 * D-UI-2, half one: the regular chrome. `/assemble/:id` is deliberately not a
 * child of this layout route — splitting the tree now is cheap, splitting it
 * during WP-B04 would not be.
 */
export function AppShell() {
  return (
    <div className="shell">
      <header className="shell__header">
        <span className="shell__brand">BeadFlow</span>
        <ThemeToggle />
      </header>
      <main className="shell__main">
        <Outlet />
      </main>
      <BottomNav />
    </div>
  );
}
