import { RouterProvider, createBrowserRouter } from "react-router";

import { StoreProvider } from "../stores/store.tsx";
import { ThemeProvider } from "../stores/theme.tsx";
import { routes } from "./routes.tsx";

const router = createBrowserRouter(routes);

export function App() {
  return (
    <ThemeProvider>
      <StoreProvider>
        <RouterProvider router={router} />
      </StoreProvider>
    </ThemeProvider>
  );
}
