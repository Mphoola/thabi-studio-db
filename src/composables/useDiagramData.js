import { ref } from "vue";
import { schemaDiagram } from "../lib/tauri/commands.js";
import { toUserDetail } from "../lib/errors.js";
import { buildGraph, plainGraph, positionsFrom } from "../lib/erLayout.js";
import { useToasts } from "./useToasts.js";
import { whyEmpty } from "./useDiagramEmpty.js";

/**
 * Reads a schema for the ER diagram and lays it out with elk (loaded on first
 * use). A plain arrangement is drawn first, so something correct is on
 * screen while elk works.
 */
export function useDiagramData({ tab, profileId, view }) {
  const toasts = useToasts();
  const loading = ref(false);
  const failed = ref("");
  const tables = ref([]);
  const foreignKeys = ref([]);
  const nodes = ref({});
  const lines = ref([]);
  const laidOut = ref(false);
  /** Why nothing came back, worked out rather than guessed at. */
  const emptyReason = ref("");
  let elk = null;

  async function arrange(graph) {
    try {
      if (!elk) {
        const ELK = (await import("elkjs/lib/elk.bundled.js")).default;
        elk = new ELK();
      }
      const placed = positionsFrom(await elk.layout(graph));
      nodes.value = placed.nodes;
      lines.value = placed.lines;
      view.setNodes(placed.nodes);
      laidOut.value = true;
    } catch (failure) {
      // The boxes are already drawn and correct; only the arrangement
      // failed, so say so rather than replace a usable picture with an error.
      laidOut.value = false;
      toasts.error("Couldn't arrange the diagram", toUserDetail(failure));
    }
    view.fit();
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
      const graph = buildGraph(diagram);
      const plain = plainGraph(graph);
      nodes.value = plain.nodes;
      lines.value = plain.lines;
      view.setNodes(plain.nodes);
      await arrange(graph);
    } catch (failure) {
      failed.value = toUserDetail(failure);
    } finally {
      loading.value = false;
    }
  }

  /** Lays out again after a box changed height (a table expanded). */
  function rearrange() {
    if (elk)
      arrange(
        buildGraph({ tables: tables.value, foreignKeys: foreignKeys.value }),
      );
  }

  return {
    loading,
    failed,
    tables,
    foreignKeys,
    nodes,
    lines,
    laidOut,
    emptyReason,
    load,
    rearrange,
  };
}
