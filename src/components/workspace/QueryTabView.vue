<script setup>
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useQueryRunner } from "../../composables/useQueryRunner.js";
import { useTabs } from "../../composables/useTabs.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
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

async function run(all = false) {
  const statements = editor.value?.runTarget(all) ?? [];
  if (!statements.length) return;
  try {
    await runner.run(props.tab, statements);
  } catch (error) {
    toasts.error("Couldn’t run the query", toUserDetail(error));
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
onMounted(() => {
  workspace.registerHandle(props.tab.id, {
    run,
    cancel,
    focus: () => editor.value?.focus(),
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
