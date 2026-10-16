<script setup>
import { computed, ref } from "vue";
import { Copy } from "lucide-vue-next";
import { copyText } from "../../lib/clipboard.js";
import { useToasts } from "../../composables/useToasts.js";
import { useAppSettings } from "../../composables/useAppSettings.js";
import { inspectText, isJsonType } from "../../lib/gridText.js";
import { base64ToHex, hexDump, imageMime } from "../../lib/binaryValue.js";
import { inTimeZone } from "../../lib/timeZones.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import SegmentedControl from "../common/SegmentedControl.vue";
import JsonTree from "./JsonTree.vue";

/** One cell's full value: text, a JSON tree, or binary as hex/base64/image. */
const open = ref(false);
const column = ref(null);
const cell = ref(null);
const mode = ref("text");
const toasts = useToasts();
const { settings } = useAppSettings();

const binary = computed(() =>
  cell.value?.kind === "binary" ? cell.value.value : null,
);
const inline = computed(() => binary.value?.inline ?? null);
const image = computed(() =>
  inline.value ? imageMime(binary.value.preview) : null,
);
const json = computed(() => {
  if (cell.value?.kind !== "value") return undefined;
  const text = cell.value.value.trim();
  if (!isJsonType(column.value?.typeName) && !/^[[{]/.test(text))
    return undefined;
  try {
    return JSON.parse(text);
  } catch {
    return undefined;
  }
});
const modes = computed(() => {
  if (binary.value)
    return [
      ...(image.value ? [{ value: "image", label: "Image" }] : []),
      { value: "hex", label: "Hex" },
      ...(inline.value ? [{ value: "base64", label: "Base64" }] : []),
    ];
  if (json.value !== undefined)
    return [
      { value: "tree", label: "Tree" },
      { value: "text", label: "Text" },
    ];
  return [];
});

const text = computed(() => {
  if (binary.value) {
    if (mode.value === "base64") return inline.value;
    return inline.value
      ? hexDump(inline.value)
      : inspectText(cell.value, column.value?.typeName);
  }
  const full = inspectText(cell.value, column.value?.typeName);
  return full === null
    ? null
    : inTimeZone(full, settings.value.displayTimeZone);
});
const description = computed(() => {
  const type = column.value?.typeName.toLowerCase() ?? "";
  return binary.value
    ? `${type} · ${binary.value.bytes.toLocaleString()} bytes`
    : type;
});

/** What Copy puts on the clipboard: full bytes when we have them. */
function copyValue() {
  if (!binary.value) return text.value ?? "";
  if (!inline.value) return `0x${binary.value.preview}`;
  return mode.value === "base64" ? inline.value : base64ToHex(inline.value);
}

async function copy() {
  try {
    await copyText(copyValue());
    toasts.info("Copied value");
  } catch {
    toasts.error("Couldn’t copy", "The clipboard isn’t available.");
  }
}

defineExpose({
  open(col, value) {
    column.value = col;
    cell.value = value;
    mode.value = modes.value[0]?.value ?? "text";
    open.value = true;
  },
});
</script>

<template>
  <BaseDialog
    v-if="column"
    v-model:open="open"
    size="lg"
    :title="column.name"
    :description="description"
  >
    <SegmentedControl
      v-if="modes.length > 1"
      v-model="mode"
      class="mb-2"
      :options="modes"
      label="Show value as"
    />
    <p v-if="text === null" class="font-mono text-grid-null italic">NULL</p>
    <div
      v-else-if="mode === 'image'"
      class="flex max-h-[55vh] justify-center overflow-auto rounded-md bg-surface-hover p-3"
    >
      <img
        :src="`data:${image};base64,${inline}`"
        :alt="`${column.name} image`"
        class="max-w-full object-contain"
      />
    </div>
    <div
      v-else-if="mode === 'tree'"
      class="selectable max-h-[55vh] overflow-auto rounded-md bg-surface-hover p-3 font-mono text-sm text-fg"
    >
      <JsonTree :value="json" />
    </div>
    <pre
      v-else
      class="selectable max-h-[55vh] overflow-auto rounded-md bg-surface-hover p-3 font-mono text-sm break-words whitespace-pre-wrap text-fg"
      >{{
        mode === "text" && json !== undefined
          ? JSON.stringify(json, null, 2)
          : text
      }}</pre>
    <p v-if="binary && !inline" class="mt-2 text-sm text-fg-subtle">
      Only the first bytes are shown here. Export or copy the rows to get every
      byte.
    </p>
    <template #footer>
      <BaseButton v-if="text !== null" :icon="Copy" @click="copy">
        {{
          binary && mode === "base64"
            ? "Copy Base64"
            : binary
              ? "Copy Hex"
              : "Copy Value"
        }}
      </BaseButton>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>
