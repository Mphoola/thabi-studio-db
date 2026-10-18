import { describe, expect, it } from "vitest";
import assert from "node:assert";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/**
 * Every Tauri command that takes an `id` is keyed by **profile** id.
 *
 * `Connections::session(profile_id)` looks that id up in a map of profiles. A
 * connection id is a different string, finds nothing, and comes back as a
 * clean `not_connected` — no error in any log, no trace, just a panel that
 * quietly draws nothing. That is expensive to debug and trivial to prevent.
 *
 * Read from the source on purpose: the mistake is a wrong *call site*, and no
 * test of the functions themselves would see it.
 */
const read = (relative) =>
  readFileSync(fileURLToPath(new URL(relative, import.meta.url)), "utf8");

/** Files whose only job is to call commands with an id. */
const CALLERS = [
  "../../src/components/explorer/DiagramView.vue",
  "../../src/components/explorer/ObjectView.vue",
];

/** The diagram's tab, and the two places that open one. */
/**
 * The argument object passed to openDiagram, and nothing wider.
 *
 * A fixed-length slice grew to include the next function, which reads the
 * connection id to ask whether a connection is live — allowed, and nothing to
 * do with what is sent as a command id.
 */
const openDiagramCall = (source) => {
  const at = source.indexOf("openDiagram({");
  assert.ok(at >= 0, "openDiagram is not called with an object");
  let depth = 0;
  for (let index = at; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(at, index + 1);
    }
  }
  return source.slice(at, at + 400);
};

describe("commands are called with a profile id", () => {
  it("the files that call commands never mention a connection id", () => {
    // Blunt on purpose. These two exist to call commands with an id and have
    // no other reason to know what a connection id is. An earlier version of
    // this test tried to match the call site with a regex, missed a bare
    // `props.connectionId`, and passed with the bug still in place.
    const offenders = CALLERS.filter((file) => /connectionId/.test(read(file)));
    expect(offenders).toEqual([]);
  });

  it("the diagram resolves its id the same way ObjectView does", () => {
    expect(read(CALLERS[0])).toMatch(/props\.profileId/);
    expect(read(CALLERS[1])).toMatch(/props\.tab\.profileId/);
  });

  it("opening a diagram passes a profile, never a connection", () => {
    for (const file of [
      "../../src/composables/useTabs.js",
      "../../src/components/explorer/useExplorerActions.js",
      "../../src/composables/useAppCommands.js",
    ]) {
      const source = read(file);
      if (!source.includes("openDiagram({")) continue;
      expect(openDiagramCall(source), file).not.toMatch(/connectionId/);
    }
  });

  it("may still read a connection id to ask whether a connection is live", () => {
    // Reading status.connectionId is how the command is told the connection
    // is up. Sending it as the id is the bug. Conflating the two here would
    // push the fix somewhere worse.
    expect(read("../../src/composables/useAppCommands.js")).toMatch(
      /status\?\.connectionId/,
    );
  });
});
