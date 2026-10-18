import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/**
 * The backup panel shows where the file will be written, and starting the
 * backup writes to that path.
 *
 * Both halves matter and they are in tension: the panel answers "where will
 * this go" on screen, and the start button must use that answer. A second file
 * picker inside the start handler re-asks the question the panel just
 * answered — which is what it did, and why clicking Create Backup appeared to
 * do nothing at all.
 */
const read = (name) =>
  readFileSync(
    fileURLToPath(
      new URL(`../../src/components/explorer/${name}`, import.meta.url),
    ),
    "utf8",
  );

/** The dialog owns the state and the handlers. */
const source = read("SqlExportDialog.vue");
/** The form owns the layout, including the line that shows the path. */
const form = read("BackupForm.vue");

/** The body of a top-level function, by its name. */
function bodyOf(name) {
  const start = source.indexOf(`async function ${name}(`);
  const plain = source.indexOf(`function ${name}(`);
  const at = start >= 0 ? start : plain;
  if (at < 0) throw new Error(`${name} is not defined in the dialog`);
  let depth = 0;
  for (let index = at; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(at, index);
    }
  }
  throw new Error(`${name} has no closing brace`);
}

describe("starting a backup", () => {
  it("writes to the path the panel is showing", () => {
    expect(bodyOf("start")).toMatch(/destination\.value/);
  });

  it("does not open a second file picker", () => {
    // The picker belongs to the Change button and nowhere else.
    expect(bodyOf("start")).not.toMatch(
      /pickSqlDestination|pickSaveDialog|save\(/,
    );
  });

  it("says where to save rather than starting with nothing", () => {
    // Without this, an empty path reaches Rust and File::create("") fails
    // with a message about the file rather than about the choice.
    expect(bodyOf("start")).toMatch(/if \(!path\)/);
  });
});

describe("choosing a different folder", () => {
  it("is the only thing that opens a picker", () => {
    expect(bodyOf("choose")).toMatch(/pickSqlDestination/);
  });

  it("keeps the chosen path rather than discarding it", () => {
    expect(bodyOf("choose")).toMatch(/destination\.value = picked/);
  });
});

describe("the path the panel shows", () => {
  it("is a real default, not an empty string", () => {
    expect(source).toMatch(/defaultBackupPath/);
    // Set when the dialog opens, so it is never blank while it is on screen.
    expect(source).toMatch(/destination\.value = await defaultBackupPath/);
  });

  it("is rendered where the user can read it", () => {
    expect(form).toMatch(/Save the backup to/);
    // Passed down rather than recomputed, so the panel and the file agree.
    expect(form).toMatch(/:title="destination"/);
    expect(source).toMatch(/:destination="destination"/);
  });
});
