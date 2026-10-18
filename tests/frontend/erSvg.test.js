import { describe, expect, it } from "vitest";
import {
  buildSvg,
  escapeXml,
  fileNameFor,
  rowsFor,
} from "../../src/lib/erSvg.js";

const tables = [
  {
    name: "users",
    columns: [
      { name: "id", dataType: "bigint", nullable: false, primaryKey: true },
      { name: "email", dataType: "text", nullable: true, primaryKey: false },
    ],
  },
];

const placed = {
  width: 300,
  height: 200,
  nodes: {
    users: { id: "users", x: 10, y: 20, width: 220, height: 90, ports: {} },
  },
  lines: [{ id: "fk", from: { x: 230, y: 65 }, to: { x: 300, y: 65 } }],
};

describe("a table's drawn rows", () => {
  it("counts what it leaves out rather than hiding it", () => {
    const wide = {
      name: "wide",
      columns: Array.from({ length: 40 }, (_, index) => ({
        name: `c${index}`,
      })),
    };
    const rows = rowsFor([wide], "wide", 14);
    expect(rows.columns).toHaveLength(14);
    // A box with no count on it looks like a table with fourteen columns.
    expect(rows.hidden).toBe(26);
  });

  it("says nothing is hidden when nothing is", () => {
    expect(rowsFor(tables, "users", 14).hidden).toBe(0);
  });

  it("is empty for a table it was not given", () => {
    expect(rowsFor(tables, "missing", 14)).toEqual({ columns: [], hidden: 0 });
  });

  /**
   * The wrong argument used to blank the whole panel with a message about
   * `find` being undefined, which named neither the function nor the argument.
   */
  it("draws an empty box rather than throwing on the wrong argument", () => {
    for (const wrong of [undefined, null, "users", 7, {}, [null]]) {
      expect(() => rowsFor(wrong, "users", 14)).not.toThrow();
      expect(rowsFor(wrong, "users", 14)).toEqual({ columns: [], hidden: 0 });
    }
  });

  it("ignores a null entry in the list", () => {
    expect(rowsFor([null, ...tables], "users", 14).columns).toHaveLength(2);
  });
});

describe("escaping", () => {
  it("makes a column name safe to put in a file", () => {
    // A table named <script> must not become markup in the exported file.
    expect(escapeXml('<script> & "x"')).toBe(
      "&lt;script&gt; &amp; &quot;x&quot;",
    );
    expect(escapeXml("it's")).toBe("it&apos;s");
  });

  it("escapes inside the drawing too", () => {
    const svg = buildSvg(
      {
        ...placed,
        nodes: { weird: { ...placed.nodes.users, id: "<img src=x>" } },
      },
      [],
    );
    expect(svg).not.toContain("<img src=x>");
    expect(svg).toContain("&lt;img src=x&gt;");
  });
});

describe("the exported file", () => {
  it("is the drawing, at the size of the drawing", () => {
    const svg = buildSvg(placed, tables);
    expect(svg).toContain('width="300"');
    expect(svg).toContain('height="200"');
    expect(svg.startsWith("<svg")).toBe(true);
    expect(svg.endsWith("</svg>")).toBe(true);
  });

  it("has every line and every box", () => {
    const svg = buildSvg(placed, tables);
    expect(svg).toContain("<line");
    expect(svg.match(/<rect/g).length).toBeGreaterThanOrEqual(3);
    expect(svg).toContain("users");
  });

  it("draws a key's column as a key", () => {
    const svg = buildSvg(placed, tables);
    // The weight is what makes a key readable at a glance.
    expect(svg).toMatch(/font-weight="700"[^>]*>id</);
    expect(svg).toContain(">email</text>");
  });

  it("survives an empty diagram rather than writing a broken file", () => {
    const svg = buildSvg({ width: 0, height: 0, nodes: {}, lines: [] }, []);
    expect(svg).toContain("<svg");
    expect(svg).toContain("</svg>");
  });

  it("names the file after the tab, with nothing a filesystem objects to", () => {
    expect(fileNameFor("shop diagram")).toBe("shop-diagram.svg");
    expect(fileNameFor("a/b:c")).toBe("a-b-c.svg");
    expect(fileNameFor(null)).toBe("diagram.svg");
  });
});

describe("a wide table is a box, unless someone asks otherwise", () => {
  const wide = {
    name: "wide",
    columns: Array.from({ length: 40 }, (_, index) => ({
      name: `c${index}`,
      dataType: "text",
    })),
  };

  it("caps at the default and counts the rest", () => {
    expect(rowsFor([wide], "wide").columns).toHaveLength(14);
  });

  /**
   * A cap with no way past it hides columns for good, which is worse than a
   * tall box: nobody can tell the columns are there at all.
   */
  it("shows every column when there is no cap", () => {
    expect(rowsFor([wide], "wide", Infinity).columns).toHaveLength(40);
    expect(rowsFor([wide], "wide", Infinity).hidden).toBe(0);
  });

  it("still counts them when a cap is given", () => {
    expect(rowsFor([wide], "wide", 5)).toEqual({
      columns: wide.columns.slice(0, 5),
      hidden: 35,
    });
  });
});
