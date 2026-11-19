<script setup>
import { Minus, Plus } from "lucide-vue-next";
import { blankRow } from "../../lib/createTable.js";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";
import TextInput from "../common/TextInput.vue";
import TypeInput from "./TypeInput.vue";

/**
 * The columns of a new table. A type is typed, with the engine's common ones
 * suggested, because no fixed list covers `VARCHAR(50)` or a domain type.
 * Key is a checkbox: a key on more than one column is ordinary.
 */
const rows = defineModel({ type: Array, required: true });
const props = defineProps({
  engine: { type: String, default: "postgres" },
  types: { type: Array, required: true },
});

function remove(index) {
  rows.value = rows.value.filter((_, i) => i !== index);
}
</script>

<template>
  <div class="flex flex-col gap-1.5">
    <div
      class="grid grid-cols-[minmax(0,1fr)_11rem_7rem_auto_auto_auto] items-center gap-2 text-xs text-fg-subtle"
    >
      <span>Name</span><span>Type</span><span>Default</span><span>Null</span
      ><span>Key</span><span />
    </div>
    <div
      v-for="(row, index) in rows"
      :key="index"
      class="grid grid-cols-[minmax(0,1fr)_11rem_7rem_auto_auto_auto] items-center gap-2"
    >
      <TextInput
        v-model="row.name"
        placeholder="name"
        :aria-label="`Column ${index + 1} name`"
        autocomplete="off"
      />
      <TypeInput
        v-model="row.type"
        :suggestions="types"
        :aria-label="`Column ${index + 1} type`"
      />
      <TextInput
        v-model="row.default"
        mono
        placeholder="none"
        :aria-label="`Column ${index + 1} default`"
        autocomplete="off"
      />
      <input
        v-model="row.nullable"
        type="checkbox"
        class="justify-self-center"
        :disabled="row.key"
        :aria-label="`Column ${index + 1} allows null`"
      />
      <input
        v-model="row.key"
        type="checkbox"
        class="justify-self-center"
        :aria-label="`Column ${index + 1} is part of the primary key`"
      />
      <IconButton
        :icon="Minus"
        size="sm"
        label="Remove Column"
        :disabled="rows.length <= 1"
        @click="remove(index)"
      />
    </div>
    <div>
      <BaseButton
        size="sm"
        variant="ghost"
        :icon="Plus"
        @click="rows = [...rows, blankRow(props.engine)]"
      >
        Add column
      </BaseButton>
    </div>
  </div>
</template>
