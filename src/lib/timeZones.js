/**
 * Showing date-time values in a chosen time zone.
 *
 * Only values that carry an offset (PostgreSQL `timestamptz`, SQL Server
 * `datetimeoffset`) name an instant; a plain `timestamp` or `datetime` has no
 * zone and is shown exactly as stored. Converting one of those would invent a
 * time the database never said.
 */

const WITH_OFFSET =
  /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})(?::(\d{2})(\.\d+)?)?\s*(Z|[+-]\d{2}(?::?\d{2})?)$/;

/** Time zones offered in Settings ("" keeps values as sent). */
export const TIME_ZONE_CHOICES = [
  { value: "", label: "As stored" },
  { value: "local", label: "This computer’s time zone" },
  { value: "UTC", label: "UTC" },
  ...(typeof Intl.supportedValuesOf === "function"
    ? Intl.supportedValuesOf("timeZone")
        .filter((zone) => zone !== "UTC")
        .map((zone) => ({ value: zone, label: zone.replaceAll("_", " ") }))
    : []),
];

/**
 * `text` shown in `zone`, or unchanged when it has no offset or no zone is
 * chosen. Keeps the value's precision (fractions of a second).
 */
export function inTimeZone(text, zone) {
  if (!zone || typeof text !== "string") return text;
  const match = WITH_OFFSET.exec(text.trim());
  if (!match) return text;
  const [, y, mo, d, h, mi, s = "00", fraction = "", offset] = match;
  const normalised = offset === "Z" ? "Z" : normaliseOffset(offset);
  const instant = new Date(`${y}-${mo}-${d}T${h}:${mi}:${s}${normalised}`);
  if (Number.isNaN(instant.getTime())) return text;
  const options = {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
    timeZoneName: "shortOffset",
    ...(zone === "local" ? {} : { timeZone: zone }),
  };
  let parts;
  try {
    parts = Object.fromEntries(
      new Intl.DateTimeFormat("en-CA", options)
        .formatToParts(instant)
        .map((p) => [p.type, p.value]),
    );
  } catch {
    return text; // An unknown zone: show what the server sent.
  }
  const name = offsetLabel(parts.timeZoneName);
  return `${parts.year}-${parts.month}-${parts.day} ${parts.hour}:${parts.minute}:${parts.second}${fraction} ${name}`;
}

function normaliseOffset(offset) {
  const sign = offset[0];
  const digits = offset.slice(1).replace(":", "");
  return `${sign}${digits.slice(0, 2)}:${(digits.slice(2) || "00").padEnd(2, "0")}`;
}

/** "GMT", "GMT+0", "GMT+2", "GMT-5:30" → "+00:00", "+02:00", "-05:30". */
function offsetLabel(name) {
  const match = /GMT(?:([+-])(\d{1,2})(?::(\d{2}))?)?/.exec(name ?? "");
  if (!match) return name;
  const [, sign = "+", hours = "0", minutes = "00"] = match;
  return `${sign}${hours.padStart(2, "0")}:${minutes}`;
}
