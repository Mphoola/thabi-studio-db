/**
 * Dragging a table to where you want it in the diagram.
 *
 * A press on a table starts a drag instead of a pan. Where tables were put is
 * kept on the tab, so it survives switching tabs and is applied again after a
 * re-layout; it isn't saved anywhere else.
 */
import { computed, ref } from "vue";
import { moveNode } from "../lib/erLayout.js";

/** Movement, in screen pixels, before a press counts as a drag, not a click. */
const DRAG_AFTER = 3;

export function useDiagramDrag({ tab, nodes, lines, zoom, view }) {
  if (!tab.diagramMoves) tab.diagramMoves = {};
  const moves = ref(tab.diagramMoves);
  const dragging = ref(null); // { id, startX, startY, originX, originY, moved }
  let swallowClick = false;

  function place(id, x, y) {
    const placed = moveNode(
      { nodes: nodes.value, lines: lines.value },
      id,
      x,
      y,
    );
    nodes.value = placed.nodes;
    lines.value = placed.lines;
  }

  function start(id, event) {
    const node = nodes.value[id];
    if (!node || event.button !== 0) return;
    event.stopPropagation();
    dragging.value = {
      id,
      startX: event.clientX,
      startY: event.clientY,
      originX: node.x,
      originY: node.y,
      moved: false,
    };
    event.currentTarget?.ownerSVGElement?.setPointerCapture?.(event.pointerId);
  }

  /** Returns true while a table is being dragged, so the pan stays still. */
  function move(event) {
    const drag = dragging.value;
    if (!drag) return false;
    const dx = event.clientX - drag.startX;
    const dy = event.clientY - drag.startY;
    if (!drag.moved && Math.hypot(dx, dy) < DRAG_AFTER) return true;
    drag.moved = true;
    place(
      drag.id,
      Math.round(drag.originX + dx / zoom.value),
      Math.round(drag.originY + dy / zoom.value),
    );
    return true;
  }

  function end() {
    const drag = dragging.value;
    if (!drag) return false;
    dragging.value = null;
    if (drag.moved) {
      const node = nodes.value[drag.id];
      tab.diagramMoves = moves.value = {
        ...moves.value,
        [drag.id]: { x: node.x, y: node.y },
      };
      view.setNodes(nodes.value, lines.value);
      // The click that ends a drag isn't a click on the table.
      swallowClick = true;
    }
    return true;
  }

  /** True once for the click that follows a drag. */
  function tookClick() {
    const took = swallowClick;
    swallowClick = false;
    return took;
  }

  /** Puts dragged tables back where they were put, after a layout. */
  function reapply() {
    for (const [id, at] of Object.entries(moves.value)) {
      if (nodes.value[id]) place(id, at.x, at.y);
    }
  }

  /** Forgets every move, for a fresh layout. */
  function reset() {
    tab.diagramMoves = moves.value = {};
  }

  /** Whether any table was dragged from where the layout put it. */
  const moved = computed(() => Object.keys(moves.value).length > 0);

  return { dragging, moved, start, move, end, tookClick, reapply, reset };
}
