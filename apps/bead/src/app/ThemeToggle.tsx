import { useTheme } from "../stores/theme.tsx";

export function ThemeToggle() {
  const { theme, toggleTheme } = useTheme();
  return (
    <button type="button" className="button" onClick={toggleTheme} aria-pressed={theme === "dark"}>
      {theme === "dark" ? "深色主题" : "浅色主题"}
    </button>
  );
}
