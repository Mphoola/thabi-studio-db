/**
 * How long a query may run and how many rows a result keeps.
 *
 * Both are app-wide defaults in Settings, and either can be overridden per
 * connection — a report connection wants a long timeout, a production one may
 * want fewer rows than the app default. A connection's own value wins; 0 (or
 * nothing set) means "use the app default".
 */

/** One of the two, resolved to a number a query can use. */
function pick(own, fallback, max) {
  const value = Number(own) > 0 ? Number(own) : Number(fallback);
  if (!Number.isFinite(value) || value <= 0) return null;
  return Math.min(value, max);
}

/**
 * @param {object} profile a connection profile, or nothing
 * @param {{ queryTimeoutSeconds?: number, maxResultRows?: number }} settings app settings
 * @returns {{ timeoutSeconds: number | null, maxRows: number | null }} null means "no limit",
 *   which is what the drivers treat as no limit at all.
 */
export function limitsFor(profile, settings) {
  return {
    timeoutSeconds: pick(
      profile?.queryTimeoutSeconds,
      settings?.queryTimeoutSeconds,
      24 * 60 * 60,
    ),
    maxRows: pick(profile?.maxResultRows, settings?.maxResultRows, 1_000_000),
  };
}

/** Whether a connection overrides either default, for showing it in the UI. */
export function overridesDefaults(profile) {
  return Boolean(
    profile && (profile.queryTimeoutSeconds || profile.maxResultRows),
  );
}

/** "10 s", "2,000 rows", or "no limit", for a field's hint. */
export function describeLimit(value, unit) {
  const number = Number(value);
  if (!number) return "no limit";
  const text = number.toLocaleString();
  return unit === "seconds"
    ? number >= 60
      ? `${Math.round(number / 60)} min`
      : `${text} s`
    : `${text} rows`;
}
