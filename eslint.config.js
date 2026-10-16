import js from "@eslint/js";
import pluginVue from "eslint-plugin-vue";
import prettier from "eslint-config-prettier";
import globals from "globals";

export default [
  { ignores: ["dist/**", "src-tauri/**", "node_modules/**"] },
  js.configs.recommended,
  ...pluginVue.configs["flat/recommended"],
  prettier,
  {
    files: ["**/*.{js,vue}"],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: { ...globals.browser },
    },
    rules: {
      "vue/multi-word-component-names": "off",
      // Workspace tabs are shared reactive objects passed as props; nested
      // updates (tab.sql, tab.database) are intended.
      "vue/no-mutating-props": ["error", { shallowOnly: true }],
      "no-unused-vars": ["error", { argsIgnorePattern: "^_" }],
      // A declaration read before it exists throws at runtime with "Cannot
      // access 'x' before initialization", which no unit test sees and the
      // build does not catch. Function declarations are hoisted, so they are
      // fine to use early.
      "no-use-before-define": [
        "error",
        { functions: false, classes: true, variables: true },
      ],
    },
  },
  {
    files: ["*.config.js", "scripts/**/*.js", "tests/**/*.js"],
    languageOptions: { globals: { ...globals.node } },
  },
];
