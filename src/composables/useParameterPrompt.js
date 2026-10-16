import { readonly, ref } from "vue";

/**
 * Asks for the values of a statement's placeholders (`:name`, `?`, `$1`)
 * before it runs. Values are bound by the driver, never spliced into SQL.
 *
 * @typedef {{ type: "text" | "number" | "null", value: string }} ParameterAnswer
 * @typedef {object} ParameterRequest
 * @property {string[]} labels  one per distinct parameter
 * @property {ParameterAnswer[]} defaults  last values used in this tab
 * @property {(answers: ParameterAnswer[] | null) => void} resolve
 */

const request = ref(/** @type {ParameterRequest | null} */ (null));

/**
 * @param {Omit<ParameterRequest, "resolve">} options
 * @returns {Promise<ParameterAnswer[] | null>} null when cancelled
 */
export function askParameters(options) {
  request.value?.resolve(null);
  return new Promise((resolve) => {
    request.value = { ...options, resolve };
  });
}

function settle(answers) {
  const current = request.value;
  request.value = null;
  current?.resolve(answers);
}

export function useParameterPrompt() {
  return { request: readonly(request), settle };
}
