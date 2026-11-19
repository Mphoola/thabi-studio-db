<script setup>
import { ArrowDown, ArrowUp } from "lucide-vue-next";
import TextInput from "../common/TextInput.vue";
import BaseButton from "../common/BaseButton.vue";
import IconButton from "../common/IconButton.vue";

/**
 * The table's columns, editable in place when the structure is being changed.
 *
 * A name and a type are typed into rather than chosen from a menu, because the
 * set of types a server has is not one this app can enumerate — VARCHAR(255)
 * and numeric(12,2) are not the same list on any two engines. The types the
 * table already uses are offered as suggestions.
 *
 * Changes are emitted as they are typed. Staging replaces a change on the same
 * column, so a whole type is one change rather than one per keystroke.
 */
const th =
  "h-7 border-b border-line bg-grid-header px-3 text-left text-xs font-semibold text-fg-muted";

defineProps({
  columns: { type: Array, required: true },
  editing: { type: Boolean, default: false },
  /** Types the table already uses, space separated, for the input's list. */
  typeNames: { type: String, default: "" },
  /** Column order can be changed (MySQL; the others have no DDL for it). */
  canMove: { type: Boolean, default: false },
});
const emit = defineEmits([
  "rename",
  "retype",
  "toggle-nullable",
  "set-default",
  "move",
  "drop",
]);
</script>

<template>
  <table class="w-full border-collapse rounded-md border border-line text-sm">
    <thead>
      <tr>
        <th :class="th" class="w-10">#</th>
        <th :class="th">Name</th>
        <th :class="th">Type</th>
        <th :class="th">Nullable</th>
        <th :class="th">Default</th>
        <th v-if="editing" :class="[th, canMove ? 'w-40' : 'w-24']">Change</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="(column, index) in columns" :key="column.name">
        <td class="h-7 border-b border-line px-3 text-fg-subtle tabular-nums">
          {{ index + 1 }}
        </td>
        <td class="font-medium">
          <TextInput
            v-if="editing"
            :model-value="column.name"
            class="h-6 min-w-0"
            :aria-label="`Rename ${column.name}`"
            autocomplete="off"
            @update:model-value="emit('rename', column, $event)"
          />
          <template v-else>{{ column.name }}</template>
        </td>
        <td class="h-7 border-b border-line px-3 font-mono text-fg-muted">
          <TextInput
            v-if="editing"
            :model-value="column.dataType"
            class="h-6 min-w-0 font-mono"
            :list="typeNames"
            :aria-label="`Type of ${column.name}`"
            autocomplete="off"
            @update:model-value="emit('retype', column, $event)"
          />
          <template v-else>{{ column.dataType }}</template>
        </td>
        <td class="text-fg-muted">
          {{ column.nullable ? "Yes" : "No" }}
        </td>
        <td
          class="h-7 max-w-64 truncate border-b border-line px-3 font-mono text-fg-muted"
        >
          <!-- An expression as the server writes it: 'new', 0, now(). Cleared
               means no default. -->
          <TextInput
            v-if="editing"
            :model-value="column.default ?? ''"
            class="h-6 min-w-0 font-mono"
            placeholder="none"
            :aria-label="`Default of ${column.name}`"
            autocomplete="off"
            @change="emit('set-default', column, $event.target.value)"
          />
          <template v-else>{{
            column.default === null ? "—" : column.default
          }}</template>
        </td>
        <td v-if="editing" class="h-7 border-b border-line px-3">
          <div class="flex items-center gap-1">
            <template v-if="canMove">
              <IconButton
                :icon="ArrowUp"
                size="sm"
                :label="`Move ${column.name} up`"
                :disabled="index === 0"
                @click="emit('move', column, -1)"
              />
              <IconButton
                :icon="ArrowDown"
                size="sm"
                :label="`Move ${column.name} down`"
                :disabled="index === columns.length - 1"
                @click="emit('move', column, 1)"
              />
            </template>
            <BaseButton
              size="sm"
              variant="ghost"
              @click="emit('toggle-nullable', column)"
            >
              {{ column.nullable ? "Not null" : "Nullable" }}
            </BaseButton>
            <BaseButton size="sm" variant="ghost" @click="emit('drop', column)">
              Drop
            </BaseButton>
          </div>
        </td>
      </tr>
    </tbody>
  </table>
</template>
