/** Shared look for text-like controls so inputs and selects line up. */
export const fieldClass =
  "h-7 w-full rounded-md border bg-surface px-2 text-base text-fg placeholder:text-fg-subtle transition-colors focus-visible:border-accent focus-visible:outline-none focus-visible:ring-3 focus-visible:ring-focus disabled:cursor-not-allowed disabled:opacity-50";

export function borderClass(error) {
  return error ? "border-danger" : "border-line-strong hover:border-fg-subtle";
}

export function describedBy(id, { error, hint }) {
  if (error) return `${id}-error`;
  if (hint) return `${id}-hint`;
  return undefined;
}
