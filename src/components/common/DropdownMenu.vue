<script setup>
import { nextTick, ref, useId } from "vue";
import { placeFloating } from "../../lib/position.js";
import KbdShortcut from "./KbdShortcut.vue";

/**
 * Menu attached to a trigger. Bind `props` from the trigger slot onto the
 * button that opens it. Arrow keys move, Return chooses, Escape closes.
 *
 * Items: { id, label, icon?, shortcut?, danger?, disabled? } or { separator: true }.
 */
const props = defineProps({
  items: { type: Array, required: true },
  label: { type: String, required: true },
  align: {
    type: String,
    default: "start",
    validator: (v) => ["start", "end"].includes(v),
  },
});
const emit = defineEmits(["select"]);

const id = useId();
const wrapper = ref(null);
const menu = ref(null);
const open = ref(false);
const position = ref({ top: 0, left: 0 });

const trigger = () => wrapper.value?.firstElementChild;
const enabledItems = () => [
  ...(menu.value?.querySelectorAll('[role="menuitem"]:not([disabled])') ?? []),
];

async function show(focus = "first", anchor = null) {
  menu.value.showPopover();
  await nextTick();
  position.value = placeFloating(
    anchor ?? trigger().getBoundingClientRect(),
    menu.value.getBoundingClientRect(),
    { align: props.align, gap: anchor ? 0 : 4 },
  );
  const items = enabledItems();
  (focus === "last" ? items.at(-1) : items[0])?.focus();
}

let returnFocus = null;

function toggle() {
  if (open.value) menu.value.hidePopover();
  else show();
}

function onToggle(event) {
  open.value = event.newState === "open";
  if (!open.value && menu.value.contains(document.activeElement)) {
    (returnFocus ?? trigger())?.focus();
    returnFocus = null;
  }
}

function onTriggerKeydown(event) {
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    show(event.key === "ArrowUp" ? "last" : "first");
  }
}

function onMenuKeydown(event) {
  const items = enabledItems();
  const index = items.indexOf(document.activeElement);
  const target = {
    ArrowDown: items[(index + 1) % items.length],
    ArrowUp: items[(index - 1 + items.length) % items.length],
    Home: items[0],
    End: items.at(-1),
  }[event.key];
  if (target) {
    event.preventDefault();
    target.focus();
  } else if (event.key === "Tab") {
    menu.value.hidePopover();
  }
}

function choose(item) {
  menu.value.hidePopover();
  emit("select", item.id);
}

/** Opens the menu at a point, for context menus. */
function openAt(x, y) {
  returnFocus = document.activeElement;
  if (open.value) menu.value.hidePopover();
  show("first", new DOMRect(x, y, 0, 0));
}
defineExpose({ openAt });

const triggerProps = {
  "aria-haspopup": "menu",
  "aria-controls": id,
  onClick: toggle,
  onKeydown: onTriggerKeydown,
};
</script>

<template>
  <span ref="wrapper" class="contents">
    <slot name="trigger" :props="{ ...triggerProps, 'aria-expanded': open }" />
  </span>
  <div
    :id="id"
    ref="menu"
    popover="auto"
    role="menu"
    :aria-label="label"
    class="m-0 min-w-44 rounded-lg border border-line bg-surface-raised p-1 text-fg shadow-popover"
    :style="{
      inset: 'auto',
      top: `${position.top}px`,
      left: `${position.left}px`,
    }"
    @toggle="onToggle"
    @keydown="onMenuKeydown"
  >
    <template v-for="(item, index) in items" :key="item.id ?? `sep-${index}`">
      <div
        v-if="item.separator"
        role="separator"
        class="mx-1 my-1 h-px bg-line"
      />
      <button
        v-else
        type="button"
        role="menuitem"
        tabindex="-1"
        :disabled="item.disabled"
        class="flex h-7 w-full items-center gap-2 rounded-md px-2 text-left text-base outline-none focus:bg-accent focus:text-accent-fg disabled:opacity-40"
        :class="item.danger ? 'text-danger-text' : 'text-fg'"
        @click="choose(item)"
      >
        <component
          :is="item.icon"
          v-if="item.icon"
          :size="14"
          :stroke-width="1.75"
          aria-hidden="true"
        />
        <span class="flex-1">{{ item.label }}</span>
        <KbdShortcut v-if="item.shortcut" :shortcut="item.shortcut" />
      </button>
    </template>
  </div>
</template>
