import { Navigate, type RouteObject } from "react-router";

import { AppShell } from "./AppShell.tsx";
import { AssemblePage } from "../pages/assemble/AssemblePage.tsx";
import { CreatePage } from "../pages/create/CreatePage.tsx";
import { CreatorPage } from "../pages/creator/CreatorPage.tsx";
import { EditPage } from "../pages/edit/EditPage.tsx";
import { ExplorePage } from "../pages/explore/ExplorePage.tsx";
import { InventoryPage } from "../pages/inventory/InventoryPage.tsx";
import { NotFoundPage } from "../pages/not-found/NotFoundPage.tsx";
import { PatternDetailPage } from "../pages/pattern/PatternDetailPage.tsx";
import { WorkspacePage } from "../pages/workspace/WorkspacePage.tsx";

/**
 * Two chromes, one route table (D-UI-2). Everything under the AppShell layout
 * route gets header + bottom nav; `/assemble/:id` sits beside it, not inside.
 */
export const routes: RouteObject[] = [
  {
    element: <AppShell />,
    children: [
      { index: true, element: <Navigate to="/explore" replace /> },
      { path: "explore", element: <ExplorePage /> },
      { path: "workspace", element: <WorkspacePage /> },
      { path: "create", element: <CreatePage /> },
      { path: "inventory", element: <InventoryPage /> },
      // D-ED-3: the pixel editor keeps the app chrome — it is a page you come
      // back to, not the immersive session `/assemble/:id` is.
      { path: "edit/:id", element: <EditPage /> },
      { path: "pattern/:id", element: <PatternDetailPage /> },
      { path: "creator/:id", element: <CreatorPage /> },
      { path: "*", element: <NotFoundPage /> },
    ],
  },
  { path: "assemble/:id", element: <AssemblePage /> },
];
