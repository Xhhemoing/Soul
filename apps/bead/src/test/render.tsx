import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { RouterProvider, createMemoryRouter } from "react-router";

import { routes } from "../app/routes.tsx";
import { createInMemoryRepository, type Repository } from "../stores/repository.ts";
import { StoreProvider } from "../stores/store.tsx";
import { ThemeProvider, type Theme } from "../stores/theme.tsx";
import type { PersistedState } from "../stores/types.ts";

export interface RenderAppOptions {
  route?: string;
  seed?: Partial<PersistedState>;
  theme?: Theme;
  /** Overrides the in-memory double, e.g. to drive a failing storage backend. */
  repository?: Repository;
}

/** R5: tests drive the real route table through a memory router, never the DOM history. */
export function renderApp({
  route = "/explore",
  seed,
  theme = "light",
  repository: override,
}: RenderAppOptions = {}) {
  const repository = override ?? createInMemoryRepository(seed);
  const router = createMemoryRouter(routes, { initialEntries: [route] });
  const user = userEvent.setup();
  const result = render(
    <ThemeProvider initial={theme}>
      <StoreProvider repository={repository}>
        <RouterProvider router={router} />
      </StoreProvider>
    </ThemeProvider>,
  );
  return { ...result, user, router, repository };
}

export function currentPath(router: ReturnType<typeof createMemoryRouter>): string {
  const { pathname, search } = router.state.location;
  return `${pathname}${search}`;
}
