import { nextTick } from "vue";
import { isModKey } from "../../lib/platform.js";

/**
 * Keyboard handling for the results grid: moving the selection, copying,
 * editing and opening a value. Every action is passed in, so this only
 * decides which key does what.
 */
export function gridKeydown({
  editing,
  selection,
  viewport,
  rowHeight,
  scroller,
  onEditKey,
  actions,
  startEdit,
  openInspector,
  openDetail,
  toggleDeleteSelected,
  editable,
}) {
  function scrollIntoView(row) {
    const el = scroller.value;
    if (!el) return;
    const top = row * rowHeight;
    const header = rowHeight;
    if (top < el.scrollTop) el.scrollTop = top;
    else if (top + rowHeight + header > el.scrollTop + el.clientHeight)
      el.scrollTop = top + rowHeight + header - el.clientHeight;
  }

  return function onKeydown(event) {
    if (editing.value) return;
    if (
      selection.move(
        event,
        Math.max(1, Math.floor(viewport.value / rowHeight) - 1),
      )
    ) {
      event.preventDefault();
      nextTick(() => scrollIntoView(selection.focus.value.row));
      return;
    }
    if (onEditKey(event)) return;
    if (isModKey(event) && event.key.toLowerCase() === "a") {
      event.preventDefault();
      selection.selectAll();
    } else if (isModKey(event) && event.key.toLowerCase() === "c") {
      event.preventDefault();
      actions.copy("tsv", event.shiftKey);
    } else if (event.key === "Enter" || event.key === "F2") {
      event.preventDefault();
      if (!startEdit()) openInspector();
    } else if (event.key === " ") {
      event.preventDefault();
      openInspector();
    } else if (event.altKey && event.key.toLowerCase() === "i") {
      event.preventDefault();
      openDetail();
    } else if (
      editable() &&
      (event.key === "Delete" || event.key === "Backspace")
    ) {
      event.preventDefault();
      toggleDeleteSelected();
    }
  };
}
