<script setup>
/**
 * Indexes, keys and constraints, listed and — while editing — changed.
 *
 * Split by what they are: IndexList for indexes and the primary key,
 * ConstraintList for foreign keys and checks. Both stage into the editor's
 * one list, so one review shows everything and one apply runs it in order.
 */
import ConstraintList from "./ConstraintList.vue";
import IndexList from "./IndexList.vue";

defineProps({
  structure: { type: Object, required: true },
  /** Only set while the Structure tab is being edited. */
  editing: { type: Boolean, default: false },
  engine: { type: String, default: "postgres" },
  profileId: { type: String, default: "" },
  tabId: { type: String, default: "" },
  schema: { type: String, default: "" },
  database: { type: String, default: "" },
});
const emit = defineEmits(["change"]);
</script>

<template>
  <div class="mt-4 flex flex-col gap-4">
    <IndexList
      :structure="structure"
      :editing="editing"
      @change="emit('change', $event)"
    />
    <ConstraintList
      :structure="structure"
      :editing="editing"
      :engine="engine"
      :profile-id="profileId"
      :tab-id="tabId"
      :schema="schema"
      :database="database"
      @change="emit('change', $event)"
    />
  </div>
</template>
