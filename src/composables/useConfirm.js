import { readonly, ref } from "vue";

/**
 * @typedef {object} ConfirmOptions
 * @property {string} title
 * @property {string} [message]
 * @property {string} [detail]  Shown in monospace, e.g. a key fingerprint.
 * @property {string} [confirmLabel]
 * @property {string} [cancelLabel]
 * @property {boolean} [destructive]
 */

const request = ref(
  /** @type {(ConfirmOptions & { resolve: (ok: boolean) => void }) | null} */ (
    null
  ),
);

/**
 * Asks the user to confirm an action. Resolves true on confirm, false on
 * cancel or Escape. Rendered by ConfirmHost, mounted once in App.vue.
 *
 * @param {ConfirmOptions} options
 * @returns {Promise<boolean>}
 */
export function confirm(options) {
  request.value?.resolve(false);
  return new Promise((resolve) => {
    request.value = { ...options, resolve };
  });
}

/** @param {boolean} ok */
function settle(ok) {
  const current = request.value;
  request.value = null;
  current?.resolve(ok);
}

export function useConfirmState() {
  return { request: readonly(request), settle };
}
