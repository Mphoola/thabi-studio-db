/**
 * Keyboard shortcut helpers. Shortcuts are written as strings such as
 * "Mod+Shift+K", where Mod is ⌘ on macOS and Ctrl elsewhere.
 */

export const isMac =
  typeof navigator !== "undefined" &&
  /Mac|iPhone|iPad/.test(navigator.platform || navigator.userAgent);

const KEY_ALIASES = {
  enter: "Enter",
  return: "Enter",
  esc: "Escape",
  escape: "Escape",
  period: ".",
  comma: ",",
  slash: "/",
  space: " ",
};

const MAC_SYMBOLS = {
  Enter: "↵",
  Escape: "⎋",
  " ": "Space",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backspace: "⌫",
};

/**
 * @param {string} shortcut
 * @returns {{ mod: boolean, ctrl: boolean, alt: boolean, shift: boolean, key: string }}
 */
export function parseShortcut(shortcut) {
  const parts = shortcut.split("+").map((part) => part.trim());
  const result = { mod: false, ctrl: false, alt: false, shift: false, key: "" };
  for (const part of parts) {
    const lower = part.toLowerCase();
    if (lower === "mod" || lower === "cmd" || lower === "meta")
      result.mod = true;
    else if (lower === "ctrl" || lower === "control") result.ctrl = true;
    else if (lower === "alt" || lower === "option") result.alt = true;
    else if (lower === "shift") result.shift = true;
    else
      result.key =
        KEY_ALIASES[lower] ?? (part.length === 1 ? part.toUpperCase() : part);
  }
  if (!result.key) throw new Error(`Shortcut "${shortcut}" has no key`);
  return result;
}

/**
 * Shortcut as shown to the user: "⇧⌘K" on macOS, "Ctrl+Shift+K" elsewhere.
 *
 * @param {string} shortcut
 * @param {boolean} [mac]
 */
export function formatShortcut(shortcut, mac = isMac) {
  const s = parseShortcut(shortcut);
  if (mac) {
    // Apple's order: Control, Option, Shift, Command.
    return [
      s.ctrl ? "⌃" : "",
      s.alt ? "⌥" : "",
      s.shift ? "⇧" : "",
      s.mod ? "⌘" : "",
      MAC_SYMBOLS[s.key] ?? s.key,
    ].join("");
  }
  return [
    s.mod || s.ctrl ? "Ctrl" : "",
    s.alt ? "Alt" : "",
    s.shift ? "Shift" : "",
    s.key,
  ]
    .filter(Boolean)
    .join("+");
}

/** Key from an event, normalised so "k" and "K" (with Shift) compare equal. */
function eventKey(event) {
  // With Shift or Option held, event.key may be a different character
  // ("<" for Shift+, on a US layout), so fall back to the physical key.
  if (event.code?.startsWith("Key")) return event.code.slice(3);
  if (event.code?.startsWith("Digit")) return event.code.slice(5);
  if (event.code === "Period") return ".";
  if (event.code === "Comma") return ",";
  if (event.code === "Slash") return "/";
  return event.key.length === 1 ? event.key.toUpperCase() : event.key;
}

/**
 * @param {string} shortcut
 * @param {KeyboardEvent} event
 * @param {boolean} [mac]
 */
export function matchesShortcut(shortcut, event, mac = isMac) {
  const s = parseShortcut(shortcut);
  const modPressed = mac ? event.metaKey : event.ctrlKey;
  const ctrlExpected = mac ? s.ctrl : s.ctrl || s.mod;
  return (
    s.mod === modPressed &&
    ctrlExpected === event.ctrlKey &&
    (mac ? true : !event.metaKey) &&
    s.alt === event.altKey &&
    s.shift === event.shiftKey &&
    s.key === eventKey(event)
  );
}
