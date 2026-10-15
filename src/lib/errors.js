/**
 * Turns anything thrown (Error, string, structured IPC error) into a short
 * message that is safe to show. Rust is responsible for sanitising its own
 * errors; this only picks the readable part.
 *
 * @param {unknown} error
 * @returns {string}
 */
export function toUserMessage(error) {
  if (!error) return "An unknown error occurred.";
  if (typeof error === "string") return error;
  if (typeof error === "object" && "message" in error && error.message) {
    return String(error.message);
  }
  return "An unknown error occurred.";
}

/**
 * Message plus the server's own detail and error code, when Rust sent them.
 *
 * @param {unknown} error
 * @returns {string}
 */
export function toUserDetail(error) {
  const parts = [toUserMessage(error)];
  if (error && typeof error === "object") {
    if (error.detail) parts.push(String(error.detail));
    if (error.code) parts.push(`Code ${error.code}`);
  }
  return parts.join(" · ");
}
