import { isModKey } from "../../lib/platform.js";
import { pasteText } from "../../lib/clipboard.js";
import { ref } from "vue";

/**
 * Cell editing and row deletion for ResultGrid when it has an `editor`
 * (useRowEdits). Changes are only staged here; the table view applies them.
 */
export function useGridEditing({
  props,
  data,
  selection,
  scroller,
  openInspector,
  actions,
}) {
  const editing = ref(null); // { row, col } while a cell editor is open

  // Selection and cell navigation speak display positions; the editor speaks
  // source columns. Only the editor call needs translating, so a hidden or
  // moved column is never written to the wrong one.
  const source = (position) => props.columnIndexes?.[position] ?? position;

  function startEdit(
    row = selection.focus.value.row,
    col = selection.focus.value.col,
  ) {
    if (!props.editor?.canEdit(data.rowAt(row) ?? [], source(col)))
      return false;
    selection.select(row, col);
    editing.value = { row, col };
    return true;
  }

  function commitEdit(value, move) {
    if (!editing.value) return;
    const { row, col } = editing.value;
    props.editor.setValue(data.rowAt(row), source(col), value);
    endEdit(move);
  }

  function endEdit(move = null) {
    if (!editing.value) return;
    const { row, col } = editing.value;
    editing.value = null;
    const next = {
      down: [row + 1, col],
      right: [row, col + 1],
      left: [row, col - 1],
    }[move];
    if (next) selection.select(next[0], next[1]);
    if (move !== null) scroller.value?.focus();
  }

  function selectedRowsCells() {
    const r = selection.active.value
      ? selection.range.value
      : { r0: selection.focus.value.row, r1: selection.focus.value.row };
    const rows = [];
    for (let i = r.r0; i <= r.r1; i += 1) rows.push(data.rowAt(i));
    return rows.filter(Boolean);
  }

  function toggleDeleteSelected() {
    props.editor.toggleDelete(selectedRowsCells());
  }

  function onMenuSelect(id) {
    const { row, col } = selection.focus.value;
    if (id === "inspect") openInspector();
    else if (id === "edit-cell") startEdit();
    else if (id === "set-null") {
      const cells = data.rowAt(row);
      if (cells && props.editor.canEdit(cells, source(col)))
        props.editor.setValue(cells, source(col), null);
    } else if (id === "set-default") {
      const cells = data.rowAt(row);
      if (cells && props.editor.canEdit(cells, source(col)))
        props.editor.setDefault(cells, source(col));
    } else if (id === "duplicate-rows") duplicateSelected();
    else if (id === "paste-rows") pasteRows();
    else if (id === "delete-rows") toggleDeleteSelected();
    else if (id === "revert-rows")
      for (const cells of selectedRowsCells()) props.editor.revert(cells);
    else actions.run(id);
  }

  function duplicateSelected() {
    props.editor?.duplicate(selectedRowsCells());
  }

  async function pasteRows() {
    if (!props.editor) return;
    try {
      props.editor.paste(await pasteText());
    } catch {
      // Nothing readable on the clipboard: nothing to paste.
    }
  }

  /**
   * Editing shortcuts while the grid has focus. Returns true when handled.
   * Mod+Z / Mod+Shift+Z undo and redo staged edits, Mod+D duplicates the
   * selected rows, Mod+V pastes clipboard rows as new rows.
   */
  function onEditKey(event) {
    if (!props.editor?.enabled.value || !isModKey(event)) return false;
    const key = event.key.toLowerCase();
    if (key === "z") event.shiftKey ? props.editor.redo() : props.editor.undo();
    else if (key === "y") props.editor.redo();
    else if (key === "d") duplicateSelected();
    else if (key === "v") pasteRows();
    else return false;
    event.preventDefault();
    return true;
  }

  return {
    onEditKey,
    editing,
    startEdit,
    commitEdit,
    endEdit,
    toggleDeleteSelected,
    onMenuSelect,
  };
}
