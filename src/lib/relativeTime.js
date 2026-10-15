/**
 * "just now", "5 min ago", "2 h ago", "yesterday", "3 days ago", or a date.
 * Accepts SQLite's UTC "YYYY-MM-DD HH:MM:SS" or an ISO string.
 */
export function relativeTime(value, now = new Date()) {
  if (!value) return "";
  const iso = /^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d$/.test(value)
    ? `${value.replace(" ", "T")}Z`
    : value;
  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) return "";
  const seconds = Math.max(0, (now - then) / 1000);
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)} min ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} h ago`;
  const days = Math.floor(seconds / 86400);
  if (days === 1) return "yesterday";
  if (days < 7) return `${days} days ago`;
  return then.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}
