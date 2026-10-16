/**
 * Turning result rows into a simple chart: one label column for the x axis
 * and one or more numeric columns as series. Pure, so it is unit-tested.
 */

/** Rows read for a chart; more would not be readable as bars anyway. */
export const CHART_ROWS = 500;

const SERIES_COLOURS = ["blue", "orange", "green", "purple", "teal", "red"];

function toNumber(cell) {
  if (cell?.kind !== "value") return null;
  const number = Number(cell.value);
  return Number.isFinite(number) ? number : null;
}

/** Columns that hold a number in most of the given rows. */
export function numericColumns(columns, rows) {
  return columns
    .map((column, index) => ({ column, index }))
    .filter(({ index }) => {
      const values = rows
        .map((row) => row[index])
        .filter((c) => c?.kind === "value");
      if (!values.length) return false;
      const numbers = values.filter((c) => toNumber(c) !== null).length;
      return numbers / values.length >= 0.9;
    });
}

/**
 * The chart's data: labels from `xIndex`, one series per `yIndexes`
 * entry, the value range (always including zero for bars), and a colour
 * token name per series.
 */
export function chartData(columns, rows, xIndex, yIndexes) {
  const labels = rows.map((row) => {
    const cell = row[xIndex];
    return cell?.kind === "value" ? cell.value : "NULL";
  });
  const series = yIndexes.map((index, i) => ({
    name: columns[index]?.name ?? "",
    colour: SERIES_COLOURS[i % SERIES_COLOURS.length],
    values: rows.map((row) => toNumber(row[index])),
  }));
  const all = series.flatMap((s) => s.values).filter((v) => v !== null);
  const min = Math.min(0, ...all);
  const max = Math.max(0, ...all);
  return { labels, series, min, max: max === min ? min + 1 : max };
}

/** A few round tick values between min and max. */
export function ticks(min, max, count = 4) {
  const span = max - min;
  const raw = span / count;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const step =
    [1, 2, 2.5, 5, 10]
      .map((m) => m * magnitude)
      .find((s) => span / s <= count) ?? raw;
  const out = [];
  for (let v = Math.ceil(min / step) * step; v <= max + step * 1e-9; v += step)
    out.push(Number(v.toPrecision(12)));
  return out;
}
