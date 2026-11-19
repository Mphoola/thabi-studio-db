import { computed, ref } from "vue";
import { schemaDiagram } from "../lib/tauri/commands.js";
import { toUserDetail } from "../lib/errors.js";
import {
  buildGraph,
  extentOf,
  gridBelow,
  plainGraph,
  positionsFrom,
  splitIsolated,
} from "../lib/erLayout.js";
import { useToasts } from "./useToasts.js";
import { whyEmpty } from "./useDiagramEmpty.js";

/**
 * Reads a schema for the ER diagram and lays it out with elk (loaded on first
 * use). A plain arrangement is drawn first, so something correct is on
 * screen while elk works.
 */
/**
 * @param limitFor table name -> rows its box draws (the cap, or Infinity when
 *   expanded), so each box is laid out at the height it is drawn
 * @param afterLayout called once elk has placed everything
 */
export function useDiagramData({
  tab,
  profileId,
  view,
  limitFor,
  afterLayout,
}) {
  const toasts = useToasts();
  const loading = ref(false);
  const failed = ref("");
  const tables = ref([]);
  const foreignKeys = ref([]);
  const nodes = ref({});
  const lines = ref([]);
  const laidOut = ref(false);
  /** Where "Tables without relationships" is written, or null. */
  const caption = ref(null);
  /** Why nothing came back, worked out rather than guessed at. */
  const emptyReason = ref("");
  let elk = null;

  async function arrange(graph, { reframe = true } = {}) {
    try {
      if (!elk) {
        const ELK = (await import("elkjs/lib/elk.bundled.js")).default;
        elk = new ELK();
      }
      // Related tables through elk; the rest in a grid beneath them.
      const { connected, isolated } = splitIsolated(graph);
      const laid = connected.children.length
        ? await elk.layout(connected)
        : { children: [], edges: [] };
      const top = connected.children.length
        ? extentOf(positionsFrom(laid).nodes).height
        : -72;
      const grid = gridBelow(
        isolated,
        top,
        extentOf(positionsFrom(laid).nodes).width,
      );
      const placed = positionsFrom({
        children: [...laid.children, ...grid.children],
        edges: laid.edges,
      });
      caption.value = grid.caption;
      nodes.value = placed.nodes;
      lines.value = placed.lines;
      // Tables the user dragged go back where they put them.
      afterLayout?.();
      view.setNodes(nodes.value, lines.value);
      laidOut.value = true;
    } catch (failure) {
      // The boxes are already drawn and correct; only the arrangement
      // failed, so say so rather than replace a usable picture with an error.
      laidOut.value = false;
      toasts.error("Couldn't arrange the diagram", toUserDetail(failure));
    }
    // Only a fresh diagram is framed; expanding a table keeps the view where
    // the user put it.
    if (reframe) view.open();
  }

  async function load() {
    if (!profileId()) {
      failed.value = "This connection is not open.";
      return;
    }
    loading.value = true;
    failed.value = "";
    laidOut.value = false;
    try {
      const diagram = await schemaDiagram(
        profileId(),
        tab.database,
        tab.schema,
      );
      tables.value = diagram.tables ?? [];
      foreignKeys.value = diagram.foreignKeys ?? [];
      emptyReason.value = "";
      if (!tables.value.length) emptyReason.value = await whyEmpty();
      const graph = buildGraph(diagram, limitFor);
      const plain = plainGraph(graph);
      nodes.value = plain.nodes;
      lines.value = plain.lines;
      view.setNodes(plain.nodes, plain.lines);
      await arrange(graph);
    } catch (failure) {
      failed.value = toUserDetail(failure);
    } finally {
      loading.value = false;
    }
  }

  /** Every foreign key column, so its row can carry an FK badge. */
  const keyed = computed(
    () =>
      new Set(
        foreignKeys.value.flatMap((key) =>
          (key.fromColumns ?? []).map((column) => `${key.fromTable}:${column}`),
        ),
      ),
  );

  /** Lays out again after a box changed height (a table expanded). */
  function rearrange() {
    if (elk)
      arrange(
        buildGraph(
          { tables: tables.value, foreignKeys: foreignKeys.value },
          limitFor,
        ),
        { reframe: false },
      );
  }

  return {
    loading,
    failed,
    tables,
    foreignKeys,
    keyed,
    nodes,
    lines,
    laidOut,
    caption,
    emptyReason,
    load,
    rearrange,
  };
}
