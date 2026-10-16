import { isModKey } from "../lib/platform.js";
import { computed, ref } from "vue";

/**
 * Rectangular cell selection with keyboard movement, like a spreadsheet.
 * Positions are display rows (after sorting) and column indexes.
 */
export function useGridSelection(rowCount, columnCount) {
  const anchor = ref({ row: 0, col: 0 });
  const focus = ref({ row: 0, col: 0 });
  const active = ref(false);

  const range = computed(() => ({
    r0: Math.min(anchor.value.row, focus.value.row),
    r1: Math.max(anchor.value.row, focus.value.row),
    c0: Math.min(anchor.value.col, focus.value.col),
    c1: Math.max(anchor.value.col, focus.value.col),
  }));

  const clamp = (cell) => ({
    row: Math.max(0, Math.min(rowCount.value - 1, cell.row)),
    col: Math.max(0, Math.min(columnCount.value - 1, cell.col)),
  });

  function select(row, col, extend = false) {
    const cell = clamp({ row, col });
    focus.value = cell;
    if (!extend) anchor.value = cell;
    active.value = true;
  }

  function selectAll() {
    anchor.value = { row: 0, col: 0 };
    focus.value = { row: rowCount.value - 1, col: columnCount.value - 1 };
    active.value = true;
  }

  function isSelected(row, col) {
    const r = range.value;
    return (
      active.value && row >= r.r0 && row <= r.r1 && col >= r.c0 && col <= r.c1
    );
  }

  /** Moves the focus for an arrow/Home/End/Page key. Returns true if handled. */
  function move(event, pageRows) {
    const { row, col } = focus.value;
    const moves = {
      ArrowUp: [row - 1, col],
      ArrowDown: [row + 1, col],
      ArrowLeft: [row, col - 1],
      ArrowRight: [row, col + 1],
      PageUp: [row - pageRows, col],
      PageDown: [row + pageRows, col],
      Home: isModKey(event) ? [0, 0] : [row, 0],
      End: isModKey(event)
        ? [rowCount.value - 1, columnCount.value - 1]
        : [row, columnCount.value - 1],
    };
    const target = moves[event.key];
    if (!target) return false;
    const vertical =
      isModKey(event) && (event.key === "ArrowUp" || event.key === "ArrowDown");
    const to = vertical
      ? [event.key === "ArrowUp" ? 0 : rowCount.value - 1, col]
      : target;
    select(to[0], to[1], event.shiftKey);
    return true;
  }

  return { anchor, focus, range, active, select, selectAll, isSelected, move };
}
