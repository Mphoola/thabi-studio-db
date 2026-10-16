/**
 * The facts the title bar shows about the selected connection, in the order
 * people scan them: server, encryption, tunnel, then where queries go.
 */

const ENGINES = {
  postgres: "PostgreSQL",
  mysql: "MySQL",
  sqlserver: "SQL Server",
};

/** "Verified TLS", "TLS, unverified" or "No TLS", from the saved setting. */
export function tlsLabel(tlsMode) {
  if (tlsMode === "verify-full") return "Verified TLS";
  if (tlsMode === "require") return "TLS, unverified";
  return "No TLS";
}

/**
 * @param {object} profile the selected connection profile
 * @param {object | null} status its live status, if any
 * @returns {{ key: string, text: string, tone?: string }[]}
 */
export function connectionSummary(profile, status) {
  const connected = status?.state === "connected";
  const server = connected
    ? status.server?.version || ENGINES[profile.driver]
    : ENGINES[profile.driver] || profile.driver;
  const parts = [{ key: "server", text: server }];

  const tls = tlsLabel(profile.tlsMode);
  parts.push({
    key: "tls",
    text: tls,
    tone: profile.tlsMode === "require" ? "warning" : undefined,
  });
  if (profile.sshEnabled) parts.push({ key: "ssh", text: "SSH" });

  parts.push({ key: "name", text: profile.name, tone: "strong" });
  if (profile.username) parts.push({ key: "user", text: profile.username });
  const database =
    (connected && status.server?.database) || profile.databaseName;
  if (database) parts.push({ key: "database", text: database });
  return parts;
}
