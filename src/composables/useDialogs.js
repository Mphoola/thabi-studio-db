import { ref } from "vue";

/** App-level dialogs, opened from commands. One open at a time. */
const open = ref(
  /** @type {null | "about" | "shortcuts" | "palette" | "connection"} */ (null),
);

export function useDialogs() {
  return {
    open,
    show: (name) => (open.value = name),
    close: () => (open.value = null),
  };
}
