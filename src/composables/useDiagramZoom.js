/**
 * Pan and zoom for the diagram.
 *
 * Split out because it is a set of gestures, not a diagram: the thing that can
 * be wrong about a database should not be in the same file as the thing that
 * can be wrong about a trackpad.
 */
import { computed, ref } from "vue";
import { extentOf } from "../lib/erLayout.js";

const MIN_ZOOM = 0.1;
const MAX_ZOOM = 2;

/** A diagram 100 tables wide is useless until it is fitted to the window. */
export function useDiagramZoom() {
  const zoom = ref(1);
  const offset = ref({ x: 24, y: 24 });
  const panning = ref(false);
  const panFrom = ref(null);
  const host = ref(null);

  /** Wraps the placed nodes so the extent can be measured as they change. */
  const nodes = ref({});
  const extent = computed(() => extentOf(nodes.value));

  function setNodes(next) {
    nodes.value = next;
  }

  function fit() {
    const box = host.value?.getBoundingClientRect();
    if (!box || !extent.value.width) return;
    const scale = Math.min(
      box.width / (extent.value.width + 48),
      box.height / (extent.value.height + 48),
      1,
    );
    zoom.value = Number.isFinite(scale) && scale > 0 ? scale : 1;
    offset.value = { x: 24, y: 24 };
  }

  function zoomBy(factor) {
    zoom.value = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom.value * factor));
  }

  function panStart(event) {
    panFrom.value = {
      x: event.clientX - offset.value.x,
      y: event.clientY - offset.value.y,
    };
    panning.value = true;
    host.value?.setPointerCapture?.(event.pointerId);
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
   * Trackpads send wheel events to scroll, and a diagram that jumps when the
   * pointer crosses it cannot be read, so zoom needs the modifier held.
   */
  function wheel(event) {
    if (!event.ctrlKey && !event.metaKey) return false;
    event.preventDefault();
    zoomBy(event.deltaY < 0 ? 1.1 : 1 / 1.1);
    return true;
  }

  return {
    host,
    zoom,
    offset,
    panning,
    extent,
    setNodes,
    fit,
    zoomBy,
    panStart,
    panMove,
    panEnd,
    wheel,
  };
}
