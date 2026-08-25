import type { BackdropKind, Project } from "../../stores/types.ts";

// D-UI-6: everything in this module reads the project record and nothing else.
// If a theme token ever leaks in here, the two colour systems have merged and
// the immersive page will fight the app chrome.

export const BACKDROP_PRESET: Record<Exclude<BackdropKind, "custom">, string> = {
  black: "#0b0b0d",
  white: "#f7f7f7",
};

export const BACKDROP_LABEL: Record<BackdropKind, string> = {
  black: "黑色背景",
  white: "白色背景",
  custom: "自定义背景",
};

export function backdropColorOf(project: Pick<Project, "backdrop" | "backdropColor">): string {
  return project.backdrop === "custom" ? project.backdropColor : BACKDROP_PRESET[project.backdrop];
}

/** Relative luminance, so text stays legible on whatever the user picked. */
export function readableTextColor(hex: string): string {
  const normalized = hex.replace("#", "");
  const full =
    normalized.length === 3
      ? normalized
          .split("")
          .map((char) => char + char)
          .join("")
      : normalized;
  if (full.length !== 6) return "#ffffff";
  const channels = [0, 2, 4].map((offset) => Number.parseInt(full.slice(offset, offset + 2), 16) / 255);
  const linear = channels.map((value) =>
    value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
  );
  const [r = 0, g = 0, b = 0] = linear;
  const luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  return luminance > 0.45 ? "#101014" : "#f5f6fa";
}
