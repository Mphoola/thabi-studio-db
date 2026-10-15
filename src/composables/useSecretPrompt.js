import { readonly, ref } from "vue";

/**
 * Asks the user for a secret at connect time, for connections set to "Ask every
 * time". The value is handed straight to the connect call and kept nowhere:
 * not in SQLite, not in the OS credential store, not in local storage.
 *
 * @typedef {object} SecretRequest
 * @property {string} title
 * @property {string} message
 * @property {string} label
 * @property {"databasePassword"|"sshPassword"|"sshKeyPassphrase"} secret
 * @property {(value: string) => void} resolve
 */

const request = ref(/** @type {SecretRequest | null} */ (null));

/**
 * @param {Omit<SecretRequest, "resolve">} options
 * @returns {Promise<string | null>} the value, or null if cancelled
 */
export function askSecret(options) {
  request.value?.resolve(null);
  return new Promise((resolve) => {
    request.value = { ...options, resolve };
  });
}

/** @param {string | null} value */
function settle(value) {
  const current = request.value;
  request.value = null;
  current?.resolve(value);
}

export function useSecretPrompt() {
  return { request: readonly(request), settle };
}

/** How each secret is named to the user. */
const LABELS = {
  databasePassword: {
    label: "Database password",
    message: (connection) =>
      `“${connection}” asks for its password each time instead of keeping it.`,
  },
  sshPassword: {
    label: "SSH password",
    message: (connection) =>
      `“${connection}” asks for its SSH password each time instead of keeping it.`,
  },
  sshKeyPassphrase: {
    label: "SSH key passphrase",
    message: (connection) =>
      `“${connection}” asks for the passphrase of its SSH key each time.`,
  },
};

/**
 * Asks for the secret a connection says it needs, worded for that secret.
 *
 * @param {{ secret: string, connection: string }} data from Rust
 * @returns {Promise<string | null>}
 */
export function askForRequiredSecret(data) {
  const known = LABELS[data.secret];
  if (!known) return Promise.resolve(null);
  return askSecret({
    title: known.label,
    message: known.message(data.connection),
    label: known.label,
    secret: data.secret,
  });
}
