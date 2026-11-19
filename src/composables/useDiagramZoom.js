/**
 * Pan and zoom for the diagram.
 *
 * Split out because it is a set of gestures, not a diagram: the thing that can
 * be wrong about a database should not be in the same file as the thing that
 * can be wrong about a trackpad.
 *
 * The screen position of a diagram point p is `offset + p * zoom`.
 */
import { computed, ref } from "vue";
import { extentOf } from "../lib/erLayout.js";

const MIN_ZOOM = 0.1;
const MAX_ZOOM = 2;
/** Smallest zoom a diagram opens at: below it, names can't be read. */
const READABLE_ZOOM = 0.6;
const MARGIN = 32;

/**
 * The offset that keeps the diagram point under `at` (screen coordinates,
 * relative to the panel) in place when the zoom changes from `from` to `to`.
 */
export function zoomAround(offset, at, from, to) {
  const ratio = to / from;
  return {
    x: at.x - (at.x - offset.x) * ratio,
    y: at.y - (at.y - offset.y) * ratio,
  };
}

/**
 * Where a diagram opens: all of it, centred, when that is still readable;
 * otherwise at a readable size from its top-left corner, to pan from there.
 */
export function openingView(box, extent) {
  const fits = Math.min(
    (box.width - 2 * MARGIN) / extent.width,
    (box.height - 2 * MARGIN) / extent.height,
    1,
  );
  const zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, fits));
  return {
    zoom,
    offset: {
      x: Math.max(MARGIN, (box.width - extent.width * zoom) / 2),
      y: Math.max(MARGIN, (box.height - extent.height * zoom) / 2),
    },
  };
}

/** A diagram 100 tables wide is useless until it is fitted to the window. */
export function useDiagramZoom() {
  const zoom = ref(1);
  const offset = ref({ x: MARGIN, y: MARGIN });
  const panning = ref(false);
  const panFrom = ref(null);
  const host = ref(null);

  /** Wraps the placed nodes so the extent can be measured as they change. */
  const nodes = ref({});
  const lines = ref([]);
  const extent = computed(() => extentOf(nodes.value, lines.value));

  function setNodes(next, nextLines = []) {
    nodes.value = next;
    lines.value = nextLines;
  }

  const box = () => host.value?.getBoundingClientRect();

  /** All of the diagram, centred (the Fit button). */
  function fit() {
    const rect = box();
    if (!rect || !extent.value.width) return;
    const view = openingView(rect, extent.value);
    zoom.value = view.zoom;
    offset.value = view.offset;
  }

  /** How it opens: fitted if readable, otherwise readable from the top left. */
  function open() {
    const rect = box();
    if (!rect || !extent.value.width) return;
    const view = openingView(rect, extent.value);
    if (view.zoom >= READABLE_ZOOM) {
      zoom.value = view.zoom;
      offset.value = view.offset;
    } else {
      zoom.value = READABLE_ZOOM;
      offset.value = { x: MARGIN, y: MARGIN };
    }
  }

  /** Zooms keeping `at` (panel coordinates; the centre by default) still. */
  function zoomBy(factor, at = null) {
    const rect = box();
    const point = at ?? {
      x: (rect?.width ?? 0) / 2,
      y: (rect?.height ?? 0) / 2,
    };
    const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom.value * factor));
    offset.value = zoomAround(offset.value, point, zoom.value, next);
    zoom.value = next;
  }

  function panStart(event) {
    if (event.button !== 0) return;
    panFrom.value = {
      x: event.clientX - offset.value.x,
      y: event.clientY - offset.value.y,
    };
    panning.value = true;
    // Captured by the element that listens for the moves: capturing on the
    // panel instead sent every move to the panel, and the drawing never
    // followed the hand.
    event.currentTarget?.setPointerCapture?.(event.pointerId);
  }

  function panMove(event) {
    if (!panFrom.value) return;
    offset.value = {
      x: event.clientX - panFrom.value.x,
      y: event.clientY - panFrom.value.y,
    };
  }

  function panEnd() {
    panFrom.value = null;
    panning.value = false;
  }

  /**
   * Two-finger scroll moves the drawing; pinch, or ⌘/Ctrl with the wheel,
   * zooms around the pointer. (Trackpad pinches arrive as wheel events with
   * Ctrl held.)
   */
  function wheel(event) {
    event.preventDefault();
    if (event.ctrlKey || event.metaKey) {
      const rect = box();
      const at = rect
        ? { x: event.clientX - rect.left, y: event.clientY - rect.top }
        : null;
      // Pinches send small deltas, mouse wheels large ones.
      const factor = Math.exp(-Math.max(-50, Math.min(50, event.deltaY)) / 100);
      zoomBy(factor, at);
      return true;
    }
    offset.value = {
      x: offset.value.x - event.deltaX,
      y: offset.value.y - event.deltaY,
    };
    return true;
  }

  /** Puts a diagram point in the middle of the panel. */
  function centreOn(point, atLeast = READABLE_ZOOM) {
    const rect = box();
    if (!rect) return;
    zoom.value = Math.max(zoom.value, atLeast);
    offset.value = {
      x: rect.width / 2 - point.x * zoom.value,
      y: rect.height / 2 - point.y * zoom.value,
    };
  }

  return {
    host,
    zoom,
    offset,
    panning,
    extent,
    setNodes,
    fit,
    open,
    zoomBy,
    centreOn,
    panStart,
    panMove,
    panEnd,
    wheel,
  };
}
