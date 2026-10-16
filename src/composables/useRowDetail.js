import { readonly, ref } from "vue";

/**
 * Opens the row detail panel from anywhere (the keyboard in the grid, or the
 * command palette) without threading a ref through the component tree: the
 * grid registers how to open itself, and the command calls it.
 */
const opener = ref(/** @type {(() => void) | null} */ (null));

export function useRowDetail() {
  return {
    /** True while a grid is on screen that can show the panel. */
    available: readonly(opener),
    /** Called by the grid so it can be opened from elsewhere. */
    set: (open) => (opener.value = open),
    open: () => opener.value?.(),
  };
}
