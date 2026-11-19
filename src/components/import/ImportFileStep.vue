<script setup>
import { FileUp } from "lucide-vue-next";
import { pickImportFile } from "../../lib/tauri/files.js";
import BaseButton from "../common/BaseButton.vue";
import CheckboxInput from "../common/CheckboxInput.vue";
import InlineAlert from "../common/InlineAlert.vue";
import SelectInput from "../common/SelectInput.vue";

/** The file, how to read it, and its first rows as read that way. */
const props = defineProps({ state: { type: Object, required: true } });
const s = props.state;

const delimiters = [
  { value: ",", label: "Comma" },
  { value: ";", label: "Semicolon" },
  { value: "\\t", label: "Tab" },
  { value: "|", label: "Pipe" },
];
const formats = [
  { value: "csv", label: "CSV or TSV" },
  { value: "json", label: "JSON" },
];
const encodings = [
  { value: "utf8", label: "UTF-8" },
  { value: "latin1", label: "Latin-1 (Windows)" },
];

async function choose() {
  const file = await pickImportFile();
  if (file) await s.choose(file);
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <div class="flex items-center gap-3">
      <BaseButton :icon="FileUp" @click="choose">Choose File…</BaseButton>
      <span class="truncate font-mono text-xs text-fg-muted">{{
        s.path.value || "No file chosen"
      }}</span>
    </div>
    <div v-if="s.path.value" class="flex flex-wrap items-end gap-3">
      <SelectInput
        v-model="s.options.format"
        label="Format"
        :options="formats"
        class="w-36"
        @update:model-value="s.reload()"
      />
      <template v-if="s.options.format === 'csv'">
        <SelectInput
          v-model="s.options.delimiter"
          label="Separated by"
          :options="delimiters"
          class="w-32"
          @update:model-value="s.reload()"
        />
        <SelectInput
          v-model="s.options.encoding"
          label="Encoding"
          :options="encodings"
          class="w-40"
          @update:model-value="s.reload()"
        />
        <CheckboxInput
          v-model="s.options.header"
          label="First row is names"
          @update:model-value="s.reload()"
        />
      </template>
    </div>
    <InlineAlert
      v-if="s.previewError.value"
      tone="danger"
      title="The file couldn't be read"
      :message="s.previewError.value"
    />
    <div
      v-else-if="s.preview.value"
      class="max-h-56 overflow-auto rounded-md border border-line"
    >
      <table class="w-full text-xs">
        <thead class="sticky top-0 bg-grid-header text-left text-fg-muted">
          <tr>
            <th
              v-for="column in s.preview.value.columns"
              :key="column"
              class="px-2 py-1 font-semibold whitespace-nowrap"
            >
              {{ column }}
            </th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="(row, i) in s.preview.value.rows.slice(0, 10)"
            :key="i"
            class="border-t border-line"
          >
            <td
              v-for="(_, c) in s.preview.value.columns"
              :key="c"
              class="max-w-48 truncate px-2 py-1 font-mono"
              :class="row[c] == null && 'text-grid-null italic'"
            >
              {{ row[c] ?? "NULL" }}
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
