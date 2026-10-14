# 0001 — Phase 1 baseline: stack, tooling and UI primitives

**Status:** accepted · **Date:** 2026-10-14

## Context

Phase 1 of `plan.md` needs a runnable Tauri 2 + Vue 3 project with
repeatable checks, a theme strategy and basic UI primitives, without
implying any database functionality.

## Decisions

1. **Scaffolded with `create-tauri-app`** (Vue template), then converted to
   JavaScript: removed `typescript`, `vue-tsc` and the `tsconfig` files, per
   plan.md's "JavaScript, not TypeScript" rule. Types are documented with JSDoc
   where they help.
2. **Tailwind CSS v4 via `@tailwindcss/vite`.** No `tailwind.config.js`; tokens
   live in CSS (`src/styles/tokens.css`) and are exposed with `@theme inline`.
3. **Semantic design tokens with light/dark values.** Components use names such
   as `bg-surface` and `text-fg-muted`, never palette colours. Dark mode is
   driven by `data-theme` on `<html>`, set by `useTheme()`, which follows the
   macOS appearance by default. Persisting a manual choice waits for app
   settings in Phase 9.
4. **Native system fonts and 13px base size** to match macOS apps. No web
   fonts are fetched.
5. **`BaseDialog` uses the native `<dialog>` element**, which provides focus
   trapping, Escape handling and modal semantics without a dependency.
   `confirm()` (promise-based) and `useToasts()` are app-wide singletons
   rendered once by `ConfirmHost` and `ToastHost`. Destructive confirms focus
   Cancel by default.
6. **Error handling in layers:** `ErrorBoundary` catches render/lifecycle
   errors and shows a recovery view; `app.config.errorHandler` and an
   `unhandledrejection` listener turn other errors into toasts.
7. **One IPC entry point** (`src/lib/tauri/ipc.js`). All commands go through
   `call()`, which normalises errors to `{ kind, message }` and fails clearly
   when run outside the Tauri webview.
8. **Least privilege from the start.** Removed the template's `greet` command
   and `tauri-plugin-opener` (unused). Capabilities are `core:default` only.
   The only command is `app_info` (name and version).
9. **Strict CSP** in `tauri.conf.json`: `default-src 'self'`, inline styles
   allowed (Vue transitions), IPC allowed. No remote origins.
10. **Tooling:** ESLint 10 flat config with `eslint-plugin-vue` recommended
    rules, Prettier (with the Tailwind class-sorting plugin), rustfmt, and
    clippy with `-D warnings`. `npm run check` runs everything.
11. **No frontend test runner yet.** Vitest will be added in Phase 3 when there
    is real logic (form validation) to test. Rust tests run via `cargo test`.

## Known limitations

- Toasts render below an open modal dialog (the dialog sits in the browser's
  top layer). Revisit in Phase 2 if it matters in practice.
- The window uses the standard macOS title bar. A unified/overlay title bar
  is a Phase 2 design decision.

## Versions at time of writing

| Component          | Version           |
| ------------------ | ----------------- |
| Rust toolchain     | 1.96.1            |
| Node.js            | 25.8.0            |
| tauri (crate)      | 2.12.4            |
| tauri-build        | 2.7.3             |
| @tauri-apps/api    | 2.12.4            |
| @tauri-apps/cli    | 2.12.3            |
| vue                | 3.5.44            |
| vite               | 8.3.4             |
| @vitejs/plugin-vue | 6.0.10            |
| tailwindcss        | 4.3.3             |
| eslint             | 10.12.0           |
| eslint-plugin-vue  | 10.11.1           |
| prettier           | 3.9.10            |
| serde / serde_json | 1.0.229 / 1.0.154 |
