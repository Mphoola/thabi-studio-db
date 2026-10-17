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
  for (const [, ...parts] of template.matchAll(/v-for="([^"]*)"/g)) {
    for (const part of parts) {
      for (const [, name] of part.matchAll(/\b(?:in|of)\s+\(?([\w$]+)/g))
        aliases.add(name);
      for (const [, name] of part.matchAll(/(?:[(,]\s*([\w$]+)\s*(?:,|\)))/g))
        aliases.add(name);
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

  // The four that were named in the template and never added to the file.
  it("declares the helpers the highlight is built from", () => {
    for (const name of ["related", "dimmed", "rowsOf"]) {
      expect(declared.has(name), `${name} is used but not declared`).toBe(true);
    }
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
