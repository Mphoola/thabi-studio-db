const DRIVERS = new Set(["mysql", "postgres", "sqlserver"]);
const DEFAULT_PORTS = { mysql: "3306", postgres: "5432", sqlserver: "1433" };
const TLS_MODES = new Set(["verify-full", "require", "disable"]);
const ENVIRONMENTS = new Set([
  "local",
  "development",
  "testing",
  "staging",
  "production",
]);

function hasControlCharacters(value) {
  return [...String(value)].some((character) => {
    const code = character.codePointAt(0);
    return code <= 31 || (code >= 127 && code <= 159);
  });
}

export function isValidHost(host) {
  return hostError(host) === "";
}

/** Field message for a host, or "" when it's valid. Mirrors Rust's normalise_host. */
export function hostError(host) {
  const value = String(host ?? "").trim();
  if (value.includes("\\"))
    return "Named instances (host\\instance) aren't supported. Enter the host and the instance's port.";
  return checkHost(value) ? "" : "Enter a valid host name or IP address.";
}

function checkHost(value) {
  if (!value || value.length > 253 || /\s/.test(value)) return false;

  const unwrapped =
    value.startsWith("[") && value.endsWith("]") ? value.slice(1, -1) : value;
  if (/^\d+(?:\.\d+){3}\.?$/.test(unwrapped))
    return isValidIpAddress(unwrapped);
  if (isValidIpAddress(unwrapped)) return true;

  const domain = unwrapped.endsWith(".") ? unwrapped.slice(0, -1) : unwrapped;
  return (
    domain.length > 0 &&
    domain
      .split(".")
      .every(
        (label) =>
          label.length > 0 &&
          label.length <= 63 &&
          /^[a-z\d_](?:[a-z\d_-]*[a-z\d_])?$/i.test(label),
      )
  );
}

export function defaultPortFor(driver) {
  return DEFAULT_PORTS[driver] ?? DEFAULT_PORTS.postgres;
}

function isValidIpAddress(value) {
  if (/^\d+(?:\.\d+){3}\.?$/.test(value)) {
    const parts = value.split(".");
    return (
      parts.length === 4 &&
      parts.every(
        (part) =>
          part.length <= 3 &&
          !(part.length > 1 && part.startsWith("0")) &&
          Number(part) <= 255,
      )
    );
  }
  if (!value.includes(":")) return false;
  try {
    return (
      new URL(`http://[${value}]/`).hostname === `[${value.toLowerCase()}]`
    );
  } catch {
    return false;
  }
}

function validatePort(value) {
  const port = Number(value);
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

export function validateConnectionProfile(profile) {
  const errors = {};
  const name = String(profile.name ?? "").trim();
  const host = String(profile.host ?? "").trim();
  const username = String(profile.username ?? "").trim();

  if (!name) errors.name = "Enter a connection name.";
  else if ([...name].length > 80) errors.name = "Use 80 characters or fewer.";

  if (!ENVIRONMENTS.has(profile.environment))
    errors.environment = "Choose an environment.";
  if (!DRIVERS.has(profile.driver))
    errors.driver = "Choose a supported database.";
  const hostMessage = hostError(host);
  if (hostMessage) errors.host = hostMessage;
  if (!TLS_MODES.has(profile.tlsMode ?? "verify-full"))
    errors.tlsMode = "Choose a TLS setting.";
  if (!validatePort(profile.port))
    errors.port = "Enter a port from 1 to 65,535.";
  if (
    [...String(profile.databaseName ?? "").trim()].length > 128 ||
    hasControlCharacters(profile.databaseName ?? "")
  ) {
    errors.databaseName = "Use a valid database name.";
  }
  if (!username) errors.username = "Enter a user name.";
  else if ([...username].length > 128 || hasControlCharacters(username)) {
    errors.username = "Use 128 characters or fewer.";
  }

  if (profile.sshEnabled) {
    if (!isValidHost(profile.sshHost))
      errors.sshHost = "Enter a valid SSH host name or IP address.";
    if (!validatePort(profile.sshPort))
      errors.sshPort = "Enter a port from 1 to 65,535.";
    if (!String(profile.sshUsername ?? "").trim())
      errors.sshUsername = "Enter an SSH user name.";
    else if (
      [...String(profile.sshUsername).trim()].length > 128 ||
      hasControlCharacters(profile.sshUsername)
    ) {
      errors.sshUsername = "Use 128 characters or fewer.";
    }
  }

  for (const [field, value] of [
    ["databasePassword", profile.databasePassword],
    ["sshPassword", profile.sshPassword],
  ]) {
    if (value && [...value].length > 4096)
      errors[field] = "Use 4,096 characters or fewer.";
  }
  if (profile.clearDatabasePassword && profile.databasePassword)
    errors.databasePassword =
      "Enter a password or choose to remove the saved password, not both.";
  if (profile.clearSshPassword && profile.sshPassword)
    errors.sshPassword =
      "Enter a password or choose to remove the saved password, not both.";

  return errors;
}
