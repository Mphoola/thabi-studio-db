import { describe, expect, it } from "vitest";
import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";

/**
 * A Vue SFC template can reference a name that does not exist. Nothing catches
 * it at build time, nothing catches it at lint time, and it throws the moment
 * the component renders — which for a panel nobody opens during development is
 * the first anyone hears of it.
 *
 * That has happened four times in this phase, always to a hand-written
 * function or a prop added to the template and not to the file. So the
 * identifiers a template names are checked against the ones the script
 * declares.
 */
const dir = fileURLToPath(
  new URL("../../src/components/explorer/", import.meta.url),
);

/** Names the script declares: imports, refs, computeds, functions, props. */
const scriptBindings = (script) => {
  const names = new Set();
  const add = (text) =>
    text.split(",").forEach((one) => names.add(one.trim().split(/[\s:]/)[0]));
  for (const [, block] of script.matchAll(/import\s*\{([^}]*)\}\s*from/g))
    add(block);
  for (const [, name] of script.matchAll(/\bconst\s+([\w$]+)\s*=/g))
    names.add(name);
  // `const { zoom, offset } = view` declares those names too.
  for (const [, block] of script.matchAll(/\bconst\s*\{([^}]*)\}\s*=/g))
    add(block);
  for (const [, name] of script.matchAll(/\blet\s+([\w$]+)\s*=/g))
    names.add(name);
  for (const [, name] of script.matchAll(/\bfunction\s+([\w$]+)/g))
    names.add(name);
  const props = script.match(/defineProps\(\{([\s\S]*?)\n\}\)/);
  if (props)
    for (const [, key] of props[1].matchAll(/^\s*([\w$]+):/gm)) names.add(key);
  const emits = script.match(/defineEmits\(\[([\s\S]*?)\]/);
  if (emits)
    for (const [, name] of emits[1].matchAll(/["']([\w:-]+)["']/g))
      names.add(name);
  return names;
};

/**
 * Helpers a template *calls*.
 *
 * v-for aliases are skipped: they are declared by the v-for itself, so asking
 * whether the script declares them asks the wrong question.
 */
const calledHelpers = (template) => {
  const names = new Set();
  const aliases = new Set();
  // Everything the loop binds: `(column, index) in rows` binds two names, and
  // `:key="index"` on the same element refers to one of them.
  for (const [, part] of template.matchAll(/v-for="([^"]*)"/g)) {
    const [, binding] = part.match(/^(.*?)\s+(?:in|of)\s/) ?? [null, ""];
    for (const name of binding.replace(/[()]/g, ",").split(",")) {
      const trimmed = name.trim();
      if (/^[A-Za-z][\w$]*$/.test(trimmed)) aliases.add(trimmed);
    }
  }

  // Vue's expressions live *inside* quoted attribute values, so those are
  // exactly what has to be read. What must not be read is everything else in
  // quotes: `fill="url(#grid)"` is a paint value, not a call to url().
  const bound = [];
  for (const [, attribute, value] of template.matchAll(
    /([:\w-]+)\s*=\s*"([^"]*)"/g,
  )) {
    if (attribute.startsWith(":") || attribute.startsWith("v-"))
      bound.push(value);
  }
  for (const [, expression] of template.matchAll(/\{\{([^}]*)\}\}/g)) {
    bound.push(expression);
  }
  const code = bound.join(" ");

  for (const [, name] of code.matchAll(
    /(?:^|[\s(={}?:!|])([a-z][\w$]*)\s*\(/g,
  )) {
    names.add(name);
  }

  // Bare references: `@select="focusTable"` passes a handler by name, and a
  // name that is not declared throws on the first click rather than on the
  // first render — which is how focusTable went missing without a warning.
  // Only a value that is *just* a name: scanning every identifier in an
  // expression also reads object keys and the English in slots, which is how
  // a check like this talks itself out of being trusted.
  for (const [, name] of template.matchAll(
    /(?:@|v-on:|:|v-bind:)[\w.-]+="([a-z][\w$]*)"/g,
  )) {
    names.add(name);
  }
  for (const name of aliases) names.delete(name);
  return names;
};

const COMPONENTS = readdirSync(dir).filter((name) => name.endsWith(".vue"));
const read = (name) => readFileSync(`${dir}${name}`, "utf8");

describe("every helper a template calls is declared", () => {
  for (const name of COMPONENTS) {
    it(`${name} calls only what it declares`, () => {
      const source = readFileSync(`${dir}${name}`, "utf8");
      const template = source.slice(source.indexOf("<template>"));
      const script = source.slice(0, source.indexOf("</script>"));
      const declared = scriptBindings(script);
      const missing = [...calledHelpers(template)].filter(
        (helper) => !declared.has(helper),
      );
      expect(
        missing,
        `${name} calls names its script does not declare`,
      ).toEqual([]);
    });
  }
});

describe("the canvas declares what its template asks for", () => {
  const source = read("DiagramCanvas.vue");
  const script = source.slice(0, source.indexOf("</script>"));
  const declared = scriptBindings(script);

  // The names that were used in the template and never added to the file:
  // related and dimmed (now in lib/erHighlight.js), and rowsOf.
  it("declares the helpers the highlight is built from", () => {
    for (const name of ["rowsOf", "focus"]) {
      expect(declared.has(name), `${name} is used but not declared`).toBe(true);
    }
  });

  it("takes the highlight from a module that is tested", () => {
    // The dimming lives in lib/erHighlight.js precisely so that a null passed
    // where a line was expected is caught by a test rather than by a click.
    expect(read("DiagramCanvas.vue")).toMatch(/erHighlight\.js/);
    expect(declared.has("related")).toBe(false);
    expect(declared.has("dimmed")).toBe(false);
  });

  it("declares the props the highlight reads", () => {
    for (const name of ["expanded", "hovered", "selected"]) {
      expect(declared.has(name), `${name} is used but not declared`).toBe(true);
    }
  });

  it("emits every event the template triggers", () => {
    for (const name of ["hover", "select", "expand"]) {
      expect(declared.has(name), `${name} is emitted but not declared`).toBe(
        true,
      );
    }
  });
});
