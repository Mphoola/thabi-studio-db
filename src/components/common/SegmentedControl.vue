<script setup>
import { useId } from "vue";

/**
 * A row of mutually exclusive options. Arrow keys move between them.
 * Use mode "tabs" when it switches views (with aria-controls), "radio"
 * when it sets a value.
 */
const props = defineProps({
  /** @type {{ value: string, label: string, icon?: object }[]} */
  options: { type: Array, required: true },
  label: { type: String, required: true },
  mode: {
    type: String,
    default: "radio",
    validator: (v) => ["radio", "tabs"].includes(v),
  },
  /** For tabs: prefix of the panel ids, e.g. "sidebar-panel" → "sidebar-panel-explorer". */
  panelIdPrefix: { type: String, default: "" },
  stretch: { type: Boolean, default: false },
});

const model = defineModel({ type: String, required: true });
const id = useId();

function onKeydown(event, index) {
  const step = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }[
    event.key
  ];
  if (!step) return;
  event.preventDefault();
  const next = (index + step + props.options.length) % props.options.length;
  model.value = props.options[next].value;
  event.currentTarget.parentElement.children[next].focus();
}
</script>

<template>
  <div
    :role="mode === 'tabs' ? 'tablist' : 'radiogroup'"
    :aria-label="label"
    class="inline-flex gap-0.5 rounded-lg bg-surface-active p-0.5"
    :class="stretch && 'flex w-full'"
  >
    <button
      v-for="(option, index) in options"
      :id="`${id}-${option.value}`"
      :key="option.value"
      type="button"
      :role="mode === 'tabs' ? 'tab' : 'radio'"
      :aria-selected="mode === 'tabs' ? model === option.value : undefined"
      :aria-checked="mode === 'radio' ? model === option.value : undefined"
      :aria-controls="
        mode === 'tabs' && panelIdPrefix
          ? `${panelIdPrefix}-${option.value}`
          : undefined
      "
      :tabindex="model === option.value ? 0 : -1"
      class="inline-flex h-6 items-center justify-center gap-1.5 rounded-md px-2.5 text-sm transition-colors"
      :class="[
        model === option.value
          ? 'bg-surface-raised text-fg shadow-popover'
          : 'text-fg-muted hover:text-fg',
        stretch && 'flex-1',
      ]"
      @click="model = option.value"
      @keydown="onKeydown($event, index)"
    >
      <component
        :is="option.icon"
        v-if="option.icon"
        :size="13"
        :stroke-width="1.75"
        aria-hidden="true"
      />
      {{ option.label }}
    </button>
  </div>
</template>
