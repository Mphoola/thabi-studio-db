<script setup>
import { onMounted, ref, useId, watch } from "vue";

/**
 * Cell editor for enum and foreign key columns: type to filter, then pick.
 * Only listed values can be chosen, so a foreign key can't point at a row
 * that doesn't exist. `picker`: { kind, nullable, table?, search(text) }.
 */
const props = defineProps({
  picker: { type: Object, required: true },
  initial: { type: String, default: null },
});
const emit = defineEmits(["commit", "cancel"]);

const input = ref(null);
const text = ref(props.initial ?? "");
const options = ref([]);
const active = ref(0);
const loading = ref(false);
const failed = ref("");
const listId = useId();
const position = ref({});
let request = 0;
let timer;

async function search(query = text.value) {
  const mine = ++request;
  loading.value = true;
  try {
    const rows = await props.picker.search(query);
    if (mine !== request) return;
    options.value = [
      ...(props.picker.nullable && !query.trim()
        ? [{ value: null, label: null }]
        : []),
      ...rows,
    ];
    failed.value = "";
    // Start on the current value when it's listed.
    const current = options.value.findIndex(
      (o) => o.value === (query ? text.value : props.initial),
    );
    active.value = Math.max(0, current);
  } catch (error) {
    if (mine === request)
      failed.value = error?.message ?? "Couldn’t load values.";
  } finally {
    if (mine === request) loading.value = false;
  }
}

watch(text, () => {
  clearTimeout(timer);
  timer = setTimeout(search, props.picker.kind === "enum" ? 0 : 200);
});

onMounted(() => {
  const rect = input.value.getBoundingClientRect();
  position.value = {
    left: `${rect.left}px`,
    top: `${rect.bottom}px`,
    minWidth: `${Math.max(rect.width, 220)}px`,
  };
  input.value.focus();
  input.value.select();
  // Show every option first, not just ones matching the current value.
  search("");
});

function choose(option, move) {
  if (option) emit("commit", option.value, move);
}

function onKeydown(event) {
  const count = options.value.length;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    if (count)
      active.value =
        (active.value + (event.key === "ArrowDown" ? 1 : -1) + count) % count;
  } else if (event.key === "Enter" || event.key === "Tab") {
    event.preventDefault();
    choose(
      options.value[active.value],
      event.key === "Tab" ? (event.shiftKey ? "left" : "right") : "down",
    );
  } else if (event.key === "Escape") {
    event.preventDefault();
    emit("cancel");
  }
  event.stopPropagation();
}
</script>

<template>
  <input
    ref="input"
    v-model="text"
    role="combobox"
    aria-autocomplete="list"
    :aria-expanded="true"
    :aria-controls="listId"
    :aria-activedescendant="options.length ? `${listId}-${active}` : undefined"
    :aria-label="
      picker.kind === 'reference'
        ? `Choose from ${picker.table}`
        : 'Choose a value'
    "
    class="block h-[26px] w-full bg-surface px-2.5 leading-[26px] text-fg outline-2 -outline-offset-2 outline-accent"
    spellcheck="false"
    @keydown="onKeydown"
    @blur="emit('cancel')"
  />
  <Teleport to="body">
    <ul
      :id="listId"
      role="listbox"
      class="fixed z-50 max-h-64 overflow-y-auto rounded-md bg-surface-raised py-1 font-mono text-sm text-fg shadow-popover"
      :style="position"
    >
      <li
        v-for="(option, index) in options"
        :id="`${listId}-${index}`"
        :key="option.value ?? '\u0000null'"
        role="option"
        :aria-selected="index === active"
        class="flex cursor-default gap-3 px-2.5 py-1"
        :class="index === active && 'bg-accent-subtle'"
        @mousedown.prevent="choose(option, 'down')"
        @mousemove="active = index"
      >
        <span v-if="option.value === null" class="text-grid-null italic"
          >NULL</span
        >
        <span v-else>{{ option.value }}</span>
        <span v-if="option.label" class="truncate font-sans text-fg-muted">
          {{ option.label }}
        </span>
      </li>
      <li
        v-if="!options.length"
        class="px-2.5 py-1 font-sans text-fg-subtle"
        role="presentation"
      >
        {{
          failed ||
          (loading
            ? "Searching…"
            : picker.kind === "reference"
              ? `No matching row in “${picker.table}”.`
              : "No matching value.")
        }}
      </li>
      <li
        v-else-if="picker.kind === 'reference' && options.length >= 50"
        class="px-2.5 py-1 font-sans text-xs text-fg-subtle"
        role="presentation"
      >
        Showing the first 50. Type to narrow.
      </li>
    </ul>
  </Teleport>
</template>
