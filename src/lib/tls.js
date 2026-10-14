/** TLS choices offered per connection. Values match the Rust profile setting. */
export const TLS_MODES = [
  { value: "verify-full", label: "Verify certificate" },
  { value: "require", label: "Encrypt without verifying" },
  { value: "disable", label: "Off" },
];

const LOOPBACK = new Set(["localhost", "127.0.0.1", "::1", "[::1]"]);

export function isLoopbackHost(host) {
  return LOOPBACK.has(
    String(host ?? "")
      .trim()
      .toLowerCase(),
  );
}

/**
 * Suggested TLS mode for a new connection. Traffic to this Mac never leaves
 * it, and local Docker databases usually have no trusted certificate, so
 * loopback hosts start with TLS off. Everything else verifies.
 */
export function suggestedTlsMode(host) {
  return isLoopbackHost(host) ? "disable" : "verify-full";
}

export function tlsHint(mode, host) {
  if (mode === "verify-full")
    return "Encrypted, and the server's certificate is checked.";
  if (mode === "require")
    return "Encrypted, but the certificate isn't checked. Use only for a server you trust.";
  return isLoopbackHost(host)
    ? "Not encrypted. Fine for a database on this Mac."
    : "Not encrypted. Passwords and data cross the network in plain text.";
}
