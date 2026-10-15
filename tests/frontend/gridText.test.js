import { describe, expect, it } from "vitest";
import {
  csvField,
  displayText,
  inspectText,
  isNumericType,
  rowsToText,
} from "../../src/lib/gridText.js";

const v = (value) => ({ kind: "value", value });

describe("grid text", () => {
  it("recognises numeric types from every engine", () => {
    for (const t of [
      "INT8",
      "numeric",
      "DECIMAL(38,9)",
      "I64",
      "Numericn",
      "double",
    ])
      expect(isNumericType(t), t).toBe(true);
    expect(isNumericType("TEXT")).toBe(false);
  });

  it("keeps cells on one line and labels NULL and binary", () => {
    expect(displayText(v("a\nb"))).toBe("a b");
    expect(displayText({ kind: "null" })).toBe("NULL");
    expect(
      displayText({ kind: "binary", value: { bytes: 2048, preview: "ab" } }),
    ).toBe("‹2,048 bytes›");
  });

  it("pretty-prints JSON in the inspector", () => {
    expect(inspectText(v('{"a":1}'), "jsonb")).toBe('{\n  "a": 1\n}');
    expect(inspectText(v("not json"), "text")).toBe("not json");
    expect(inspectText({ kind: "null" })).toBeNull();
  });

  it("matches the Rust CSV rules", () => {
    expect(csvField("a,b")).toBe('"a,b"');
    expect(csvField('say "hi"')).toBe('"say ""hi"""');
    expect(csvField("plain")).toBe("plain");
  });

  it("formats rows as TSV with an optional header", () => {
    const columns = [{ name: "id" }, { name: "note" }];
    const rows = [
      [v("1"), v("a\tb")],
      [v("2"), { kind: "null" }],
    ];
    expect(rowsToText(columns, rows, { includeHeader: true })).toBe(
      "id\tnote\n1\ta b\n2\t\n",
    );
  });
});
