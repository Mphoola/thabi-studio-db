import { readonly, ref } from "vue";

/**
 * @typedef {"info" | "success" | "error"} ToastTone
 * @typedef {{ label: string, run: () => void }} ToastAction
 * @typedef {{ id: number, tone: ToastTone, title: string, detail?: string, action?: ToastAction }} Toast
 */

const toasts = ref(/** @type {Toast[]} */ ([]));
let nextId = 1;

const DURATION_MS = { info: 4000, success: 4000, error: 8000 };

/**
 * @param {ToastTone} tone
 * @param {string} title
 * @param {string} [detail]
 */
function push(tone, title, detail, action) {
  const id = nextId++;
  toasts.value.push({ id, tone, title, detail, action });
  setTimeout(() => dismiss(id), DURATION_MS[tone]);
  return id;
}

/** @param {number} id */
function dismiss(id) {
  toasts.value = toasts.value.filter((toast) => toast.id !== id);
}

export function useToasts() {
  return {
    toasts: readonly(toasts),
    dismiss,
    info: (title, detail, action) => push("info", title, detail, action),
    success: (title, detail, action) => push("success", title, detail, action),
    error: (title, detail, action) => push("error", title, detail, action),
  };
}
