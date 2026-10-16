import { describe, expect, it } from "vitest";
import { chartData, numericColumns, ticks } from "../../src/lib/resultChart.js";

const v = (value) => ({ kind: "value", value });
const columns = [{ name: "day" }, { name: "total" }, { name: "note" }];
const rows = [
  [v("Mon"), v("10.5"), v("a")],
  [v("Tue"), v("-2"), v("b")],
  [v("Wed"), { kind: "null" }, v("c")],
];

describe("result charts", () => {
  it("finds numeric columns", () => {
    expect(numericColumns(columns, rows).map((c) => c.index)).toEqual([1]);
  });

  it("builds labels, series and a range that includes zero", () => {
    const data = chartData(columns, rows, 0, [1]);
    expect(data.labels).toEqual(["Mon", "Tue", "Wed"]);
    expect(data.series[0]).toMatchObject({
      name: "total",
      colour: "blue",
      values: [10.5, -2, null],
    });
    expect([data.min, data.max]).toEqual([-2, 10.5]);
  });

  it("picks round ticks", () => {
    expect(ticks(0, 100)).toEqual([0, 25, 50, 75, 100]);
    expect(ticks(-2, 10.5)).toEqual([0, 5, 10]);
  });
});
