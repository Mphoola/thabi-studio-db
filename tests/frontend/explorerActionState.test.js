import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/**
 * The backup dialog is opened by a context menu and rendered by a different
 * component. Both call `useExplorerActions()`, and they only agree if the state
 * that connects them lives at module scope.
 *
 * They did not. The refs were created inside the composable, so each caller
 * got its own: the menu set one, the panel watched the other, and choosing
 * Create Backup did nothing at all — with no error anywhere, because nothing
 * had failed.
 *
 * This reads the source because the fault is about where a ref is *declared*.
 */
const source = readFileSync(
  fileURLToPath(
    new URL(
      "../../src/components/explorer/useExplorerActions.js",
      import.meta.url,
    ),
  ),
  "utf8",
);

/** Whether a declaration is inside the composable function or not. */
function declaredInsideComposable(name) {
  const body = source.slice(
    source.indexOf("export function useExplorerActions("),
  );
  const pattern = new RegExp(`\\n\\s*(const|let)\\s+${name}\\s*=\\s*ref\\(`);
  return pattern.test(body);
}

describe("the state that connects the menu to the dialog", () => {
  it("is not declared inside the composable", () => {
    // Two callers, two sets of refs, and nothing on screen happens.
    expect(declaredInsideComposable("target")).toBe(false);
    expect(declaredInsideComposable("open")).toBe(false);
    expect(declaredInsideComposable("exportTarget")).toBe(false);
    expect(declaredInsideComposable("exportOpen")).toBe(false);
  });

  it("is declared once, at module scope", () => {
    const declarations = [
      ...source.matchAll(/^const\s+(\w+)\s*=\s*ref\(/gm),
    ].map((match) => match[1]);
    for (const name of ["exportTarget", "exportOpen"]) {
      expect(declarations).toContain(name);
    }
    // Declared exactly once: a second one anywhere is the same bug again.
    expect(declarations.filter((name) => name === "exportTarget")).toHaveLength(
      1,
    );
    expect(declarations.filter((name) => name === "exportOpen")).toHaveLength(
      1,
    );
  });

  it("is handed back under the names the panel binds to", () => {
    expect(source).toMatch(/target:\s*exportTarget/);
    expect(source).toMatch(/open:\s*exportOpen/);
  });
});

describe("who sets it, and who reads it", () => {
  it("the menu action opens it", () => {
    const body = source.slice(source.indexOf("function emitExport"));
    expect(body).toMatch(/exportTarget\.value\s*=/);
    expect(body).toMatch(/exportOpen\.value\s*=\s*true/);
  });

  it("the panel reads exactly those names", () => {
    const panel = readFileSync(
      fileURLToPath(
        new URL(
          "../../src/components/explorer/ExplorerPanel.vue",
          import.meta.url,
        ),
      ),
      "utf8",
    );
    expect(panel).toMatch(/exportActions\.target\.value/);
    expect(panel).toMatch(/exportActions\.open\.value/);
  });
});
