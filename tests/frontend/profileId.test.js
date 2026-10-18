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

/** Command wrappers whose arguments are checked. */
const COMMANDS = [
  "call",
  "listObjects",
  "listDatabases",
  "listSchemas",
  "describeObject",
  "openObjectData",
  "exportSql",
  "exportTable",
  "exportCsv",
  "schemaDiagram",
  "previewTableDdl",
  "applyTableDdl",
];

/** The argument list of every command call in a source file. */
function commandCalls(source) {
  const found = [];
  for (const name of COMMANDS) {
    let from = 0;
    for (;;) {
      const at = source.indexOf(`${name}(`, from);
      if (at < 0) break;
      from = at + name.length;
      let depth = 0;
      for (let index = at + name.length; index < source.length; index += 1) {
        if (source[index] === "(") depth += 1;
        if (source[index] === ")") {
          depth -= 1;
          if (depth === 0) {
            found.push(source.slice(at + name.length, index));
            break;
          }
        }
      }
    }
  }
  return found;
}

/**
 * Files that call commands with an id.
 *
 * Every one of them, not the ones this fault has happened in. ExplorerPanel
 * passes a connection id to list_objects while the backup dialog beside it
 * passes a profile id, and the result was a panel that said "0 tables" about a
 * database full of them.
 */
const CALLERS = [
  "../../src/components/explorer/DiagramView.vue",
  "../../src/components/explorer/ObjectView.vue",
  "../../src/components/explorer/ExplorerPanel.vue",
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
  it("never passes a connection id as a command's id", () => {
    // Not "never mentions connectionId": reading status.connectionId to ask
    // whether a connection is live is correct and several files do it. What
    // must never happen is that value reaching a command's id argument, which
    // finds no session and reads as an empty result rather than an error.
    const offenders = [];
    for (const file of CALLERS) {
      for (const call of commandCalls(read(file))) {
        if (/connectionId/.test(call))
          offenders.push(`${file}: ${call.slice(0, 60)}`);
      }
    }
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
