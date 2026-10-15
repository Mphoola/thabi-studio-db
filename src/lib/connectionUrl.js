/**
 * Reads a connection URL into connection form fields:
 *   postgres://user:pass@host:5432/db?sslmode=require
 *   mysql://user@host/db
 *   sqlserver://user:pass@host:1433/db   (also mssql://)
 * Returns null when it isn't a URL we understand.
 */
const DRIVERS = {
  postgres: "postgres",
  postgresql: "postgres",
  mysql: "mysql",
  mariadb: "mysql",
  sqlserver: "sqlserver",
  mssql: "sqlserver",
};
const PORTS = { postgres: "5432", mysql: "3306", sqlserver: "1433" };
const TLS = {
  disable: "disable",
  allow: "disable",
  prefer: "require",
  require: "require",
  "verify-ca": "verify-full",
  "verify-full": "verify-full",
  required: "require",
  verify_identity: "verify-full",
  verify_ca: "verify-full",
  disabled: "disable",
  preferred: "require",
};

export function parseConnectionUrl(text) {
  const raw = String(text ?? "")
    .trim()
    .replace(/^jdbc:/i, "");
  const scheme = /^([a-z0-9+]+):\/\//i.exec(raw)?.[1]?.toLowerCase();
  const driver = DRIVERS[scheme?.split("+")[0]];
  if (!driver) return null;

  let url;
  try {
    // Parse as http so the URL API handles credentials, IPv6 and ports.
    url = new URL(raw.replace(/^[^:]+:/, "http:"));
  } catch {
    return null;
  }
  if (!url.hostname) return null;

  const params = url.searchParams;
  const sslmode =
    params.get("sslmode") ?? params.get("ssl-mode") ?? params.get("ssl_mode");
  const encrypt = params.get("encrypt");
  let tlsMode = sslmode ? TLS[sslmode.toLowerCase()] : null;
  if (!tlsMode && encrypt)
    tlsMode = /^(false|no|0)$/i.test(encrypt) ? "disable" : "verify-full";

  const decode = (value) => {
    try {
      return decodeURIComponent(value);
    } catch {
      return value;
    }
  };

  return {
    driver,
    host: url.hostname.replace(/^\[|\]$/g, ""),
    port: url.port || PORTS[driver],
    username: decode(url.username),
    databasePassword: decode(url.password),
    databaseName: decode(url.pathname.replace(/^\//, "").split("/")[0] ?? ""),
    ...(tlsMode ? { tlsMode } : {}),
  };
}
