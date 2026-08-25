import js from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

// The rule worth reading is the last one. Everything above it is house style;
// `no-restricted-imports` is the WP09 brief in linter form: the WebView holds
// no business logic, so exactly one module may speak to the core.
export default tseslint.config(
  { ignores: ["dist/**", "node_modules/**", "src-tauri/**", "coverage/**"] },
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
              group: ["@tauri-apps/api", "@tauri-apps/api/*"],
              message:
                "只有 src/core.ts 可以直接调用核心。别的模块从 core.ts 导入，界面里不要出现 IPC。",
            },
          ],
        },
      ],
    },
  },
  {
    // The boundary itself, and the tests that stand in for the far side of it.
    files: ["src/core.ts", "src/test/**/*.ts", "**/*.test.ts", "**/*.test.tsx"],
    rules: { "no-restricted-imports": "off" },
  },
  {
    files: ["**/*.test.ts", "**/*.test.tsx", "src/test/**/*.ts"],
    languageOptions: { globals: { ...globals.node } },
  },
  {
    files: ["vite.config.ts", "eslint.config.js"],
    languageOptions: { globals: { ...globals.node } },
  },
);
