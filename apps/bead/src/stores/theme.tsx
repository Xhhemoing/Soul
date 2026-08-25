import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";

// D-UI-6: this is the *app* theme and nothing else. The assemble backdrop is a
// per-project field on the project record; the two systems never read each
// other, which is why there is no backdrop state anywhere in this file.

export type Theme = "light" | "dark";

export const THEME_STORAGE_KEY = "bead.theme";

interface ThemeValue {
  theme: Theme;
  setTheme(theme: Theme): void;
  toggleTheme(): void;
}

const ThemeContext = createContext<ThemeValue | null>(null);

function readStoredTheme(): Theme | null {
  try {
    const stored = globalThis.localStorage?.getItem(THEME_STORAGE_KEY);
    return stored === "light" || stored === "dark" ? stored : null;
  } catch {
    return null;
  }
}

export function ThemeProvider({ children, initial }: { children: ReactNode; initial?: Theme }) {
  const [theme, setThemeState] = useState<Theme>(() => initial ?? readStoredTheme() ?? "light");

  useEffect(() => {
    document.documentElement.dataset["theme"] = theme;
    try {
      globalThis.localStorage?.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // A blocked storage quota must not stop the theme from applying.
    }
  }, [theme]);

  const setTheme = useCallback((next: Theme) => setThemeState(next), []);
  const toggleTheme = useCallback(
    () => setThemeState((current) => (current === "light" ? "dark" : "light")),
    [],
  );

  const value = useMemo<ThemeValue>(
    () => ({ theme, setTheme, toggleTheme }),
    [theme, setTheme, toggleTheme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeValue {
  const value = useContext(ThemeContext);
  if (value === null) throw new Error("useTheme 必须在 <ThemeProvider> 内使用");
  return value;
}
