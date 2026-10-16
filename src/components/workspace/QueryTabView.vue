<script setup>
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { loadCompletionSchema } from "../../composables/useCompletionSchema.js";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useQueryRunner } from "../../composables/useQueryRunner.js";
import { useTabs } from "../../composables/useTabs.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { explainStatement } from "../../lib/tauri/commands.js";
import { confirm } from "../../composables/useConfirm.js";
import QueryToolbar from "./QueryToolbar.vue";
import ResultsPanel from "./ResultsPanel.vue";
import SqlEditor from "./SqlEditor.vue";

/** One query tab: toolbar, editor and results, with a resizable split. */
const props = defineProps({ tab: { type: Object, required: true } });

const store = useConnectionProfiles();
const runner = useQueryRunner();
const toasts = useToasts();
const editor = ref(null);
const split = ref(0.45); // editor share of the height
const container = ref(null);

const engine = computed(
  () =>
    store.profiles.value.find((p) => p.id === props.tab.profileId)?.driver ??
    "postgres",
);

// Tables and columns for autocomplete, once the tab's connection is open.
const tables = ref([]);
watch(
  () => [
    props.tab.profileId,
    props.tab.database,
    store.isConnected(props.tab.profileId),
  ],
  async ([profileId, database, connected]) => {
    if (!connected) return;
    const loaded = await loadCompletionSchema(
      profileId,
      props.tab.id,
      database,
    );
    if (props.tab.profileId === profileId) tables.value = loaded;
  },
  { immediate: true },
);

async function run(all = false) {
  const statements = editor.value?.runTarget(all) ?? [];
  if (!statements.length) return;
  try {
    await runner.run(props.tab, statements);
  } catch (error) {
    toasts.error("Couldn’t run the query", toUserDetail(error));
  }
}

/**
 * Shows the plan for the statement under the cursor as a result tab.
 * ANALYZE (PostgreSQL) really runs it, rolled back afterwards, so it asks.
 */
async function explain(analyze = false) {
  const sql = editor.value?.currentStatement() ?? "";
  if (!sql || props.tab.running || !props.tab.profileId) return;
  if (
    analyze &&
    !(await confirm({
      title: "Run with ANALYZE?",
      message:
        "The statement really runs to measure it, then everything it changed is rolled back. Side effects outside the database (like sequences or calls to other systems) aren't undone.",
      confirmLabel: "Run and Roll Back",
    }))
  )
    return;
  const entry = { kind: "plan", sql, status: "running", analyzed: analyze };
  props.tab.runs = [entry];
  props.tab.activeRun = 0;
  const shown = props.tab.runs[0];
  try {
    if (!store.isConnected(props.tab.profileId))
      await store.connect(props.tab.profileId);
    shown.plan = await explainStatement(props.tab.profileId, props.tab.id, {
      database: props.tab.database,
      sql,
      analyze,
    });
    shown.status = "completed";
  } catch (error) {
    shown.status = "failed";
    shown.error = error;
  }
}

async function cancel() {
  try {
    await runner.cancel(props.tab);
  } catch (error) {
    toasts.error("Couldn’t cancel the query", toUserDetail(error));
  }
}

function startResize(event) {
  const box = container.value.getBoundingClientRect();
  const handle = event.currentTarget;
  handle.setPointerCapture(event.pointerId);
  const move = (e) => {
    split.value = Math.min(
      0.85,
      Math.max(0.15, (e.clientY - box.top) / box.height),
    );
  };
  const up = () => {
    handle.removeEventListener("pointermove", move);
    handle.removeEventListener("pointerup", up);
  };
  handle.addEventListener("pointermove", move);
  handle.addEventListener("pointerup", up);
}

function onResizeKey(event) {
  const step = { ArrowUp: -0.05, ArrowDown: 0.05 }[event.key];
  if (!step) return;
  event.preventDefault();
  split.value = Math.min(0.85, Math.max(0.15, split.value + step));
}

const workspace = useTabs();
// Re-reading the result after an edit goes through here; the results panel is
// what asks, since that is where staged edits are applied from.
props.tab.rerunQuery = () => run();
onMounted(() => {
  workspace.registerHandle(props.tab.id, {
    run,
    cancel,
    focus: () => editor.value?.focus(),
    format: () => editor.value?.format(),
    explain: () => explain(false),
    explainAnalyze: () => explain(true),
    minify: () => editor.value?.minify(),
  });
  editor.value?.focus();
});
onBeforeUnmount(() => workspace.unregisterHandle(props.tab.id));
</script>

<template>
  <section class="flex min-h-0 flex-col" :aria-label="tab.title">
    <QueryToolbar
      :tab="tab"
      @run="run()"
      @run-all="run(true)"
      @cancel="cancel"
    />
    <div ref="container" class="flex min-h-0 flex-1 flex-col">
      <div class="min-h-0" :style="{ height: `${split * 100}%` }">
        <SqlEditor
          ref="editor"
          v-model="tab.sql"
          :engine="engine"
          :label="`${tab.title} SQL`"
          :tables="tables"
          @run="run()"
          @run-all="run(true)"
          @cancel="cancel"
        />
      </div>
      <div
        role="separator"
        aria-orientation="horizontal"
        aria-label="Resize editor and results"
        tabindex="0"
        class="h-1.5 shrink-0 cursor-row-resize border-y border-line bg-toolbar hover:bg-accent-subtle focus-visible:bg-accent-subtle"
        @pointerdown="startResize"
        @keydown="onResizeKey"
      />
      <ResultsPanel :tab="tab" class="min-h-0 flex-1" />
    </div>
  </section>
</template>
