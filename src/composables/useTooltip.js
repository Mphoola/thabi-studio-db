import { ref } from "vue";

/**
 * One shared tooltip for the whole app, rendered by TooltipHost.
 * Use the directive: v-tooltip="'Text'" or v-tooltip="{ text, shortcut }".
 */
const state = ref(
  /** @type {{ text: string, shortcut?: string, anchor: DOMRect } | null} */ (
    null
  ),
);
const DELAY_MS = 500;
let timer = 0;

function normalise(value) {
  return typeof value === "string" ? { text: value } : (value ?? { text: "" });
}

function show(el, immediate) {
  clearTimeout(timer);
  const value = el._tooltip;
  if (!value?.text) return;
  const open = () =>
    (state.value = { ...value, anchor: el.getBoundingClientRect() });
  if (immediate || state.value) open();
  else timer = window.setTimeout(open, DELAY_MS);
}

function hide() {
  clearTimeout(timer);
  state.value = null;
}

export const vTooltip = {
  mounted(el, binding) {
    el._tooltip = normalise(binding.value);
    el._tooltipHandlers = {
      mouseenter: () => show(el, false),
      mouseleave: hide,
      focus: () => el.matches(":focus-visible") && show(el, true),
      blur: hide,
      pointerdown: hide,
      keydown: (event) => event.key === "Escape" && hide(),
    };
    for (const [name, handler] of Object.entries(el._tooltipHandlers)) {
      el.addEventListener(name, handler);
    }
  },
  updated(el, binding) {
    el._tooltip = normalise(binding.value);
  },
  beforeUnmount(el) {
    for (const [name, handler] of Object.entries(el._tooltipHandlers ?? {})) {
      el.removeEventListener(name, handler);
    }
    hide();
  },
};

export function useTooltipState() {
  return { state };
}
