<script setup>
import { computed, ref, watch } from "vue";
import { Eye, RefreshCw, Table2 } from "lucide-vue-next";
import { useExplorer } from "../../composables/useExplorer.js";
import { describeObject, previewObject } from "../../lib/tauri/commands.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";
import DataPreview from "./DataPreview.vue";
import StructureView from "./StructureView.vue";

/** A table or view opened from the explorer: read-only data or structure. */
const explorer = useExplorer();
const opened = explorer.openObject;
const node = computed(() => opened.value?.node);
const tab = computed({
  get: () => opened.value?.tab ?? "data",
  set: (value) => (opened.value = { ...opened.value, tab: value }),
});
const tabs = [
  { value: "data", label: "Data" },
  { value: "structure", label: "Structure" },
];

const results = ref({}); // tab -> { status, value?, error? }

async function load(which = tab.value) {
  const target = node.value;
  const id = explorer.profile.value?.id;
  if (!target || !id) return;
  results.value = { ...results.value, [which]: { status: "loading" } };
  try {
    const value =
      which === "data"
        ? await previewObject(id, target)
        : await describeObject(id, target);
    if (node.value === target)
      results.value = {
        ...results.value,
        [which]: { status: "loaded", value },
      };
  } catch (error) {
    if (node.value === target)
      results.value = { ...results.value, [which]: { status: "error", error } };
  }
}

watch(node, () => {
  results.value = {};
  load();
});
watch(tab, (value) => !results.value[value] && load(value));
if (node.value) load();

const current = computed(() => results.value[tab.value] ?? { status: "idle" });
const location = computed(() =>
  [node.value?.database, node.value?.schema].filter(Boolean).join(" · "),
);
</script>

<template>
  <section v-if="node" class="flex h-full min-h-0 flex-col">
    <header
      class="flex h-11 shrink-0 items-center gap-3 border-b border-line bg-toolbar px-4"
    >
      <component
        :is="node.kind === 'table' ? Table2 : Eye"
        :size="16"
        :stroke-width="1.75"
        class="text-fg-subtle"
        aria-hidden="true"
      />
      <div class="flex min-w-0 flex-col">
        <h1 class="truncate text-base font-semibold">{{ node.name }}</h1>
        <span class="truncate text-xs text-fg-subtle">{{ location }}</span>
      </div>
      <SegmentedControl
        v-model="tab"
        class="ml-auto"
        :options="tabs"
        label="Object view"
      />
      <IconButton
        :icon="RefreshCw"
        label="Reload"
        :disabled="current.status === 'loading'"
        @click="load()"
      />
    </header>
    <div
      v-if="current.status === 'loading' || current.status === 'idle'"
      class="flex flex-1 items-center justify-center gap-2 text-fg-subtle"
      role="status"
    >
      <SpinnerIcon :size="14" />
      {{ tab === "data" ? "Reading rows…" : "Reading structure…" }}
    </div>
    <div v-else-if="current.status === 'error'" class="p-4">
      <InlineAlert
        tone="danger"
        :title="current.error?.message ?? 'Couldn’t read this object.'"
        :message="current.error?.detail ?? ''"
        mono
      >
        <BaseButton size="sm" @click="load()">Try Again</BaseButton>
      </InlineAlert>
    </div>
    <DataPreview
      v-else-if="tab === 'data'"
      class="min-h-0 flex-1"
      :result="current.value"
    />
    <StructureView
      v-else
      class="min-h-0 flex-1"
      :structure="current.value"
      :kind="node.kind"
    />
  </section>
</template>
