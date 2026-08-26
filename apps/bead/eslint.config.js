import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

// BeadFlow lives in an isolated tree: it may not reach into Soul's app or its
// core. The import ban is that rule in linter form.
export default tseslint.config(
  { ignores: ["dist/**", "node_modules/**", "coverage/**"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2022,
      globals: { ...globals.browser, ...globals.es2022 },
    },
    plugins: { "react-hooks": reactHooks },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "no-restricted-imports": [
        "error",
        {
          patterns: [
            {
              group: ["@soul/*", "@tauri-apps/*", "**/apps/desktop/**"],
              message:
                "apps/bead 是隔离树：不要引用 @soul/desktop、Tauri API 或 Soul 的任何源码。",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["**/*.test.ts", "**/*.test.tsx", "src/test/**/*.{ts,tsx}"],
    languageOptions: { globals: { ...globals.node } },
  },
  {
    files: ["vite.config.ts", "eslint.config.js"],
    languageOptions: { globals: { ...globals.node } },
  },
);
