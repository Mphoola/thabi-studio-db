import { describe, expect, it } from "vitest";
import {
  formatShortcut,
  matchesShortcut,
  parseShortcut,
} from "../../src/lib/shortcuts.js";

const key = (code, key, mods = {}) => ({
  code,
  key,
  metaKey: false,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  ...mods,
});

describe("parseShortcut", () => {
  it("reads modifiers and key", () => {
    expect(parseShortcut("Mod+Shift+k")).toEqual({
      mod: true,
      ctrl: false,
      alt: false,
      shift: true,
      key: "K",
    });
  });

  it("maps named keys", () => {
    expect(parseShortcut("Mod+Period").key).toBe(".");
    expect(parseShortcut("Mod+Enter").key).toBe("Enter");
  });

  it("rejects a shortcut with no key", () => {
    expect(() => parseShortcut("Mod+Shift")).toThrow();
  });
});

describe("formatShortcut", () => {
  it("uses Apple symbols in Apple's order on macOS", () => {
    expect(formatShortcut("Shift+Mod+K", true)).toBe("⇧⌘K");
    expect(formatShortcut("Mod+Alt+I", true)).toBe("⌥⌘I");
    expect(formatShortcut("Mod+Enter", true)).toBe("⌘↵");
  });

  it("uses Ctrl elsewhere", () => {
    expect(formatShortcut("Mod+Shift+K", false)).toBe("Ctrl+Shift+K");
  });
});

describe("matchesShortcut", () => {
  it("matches Command on macOS", () => {
    expect(
      matchesShortcut("Mod+1", key("Digit1", "1", { metaKey: true }), true),
    ).toBe(true);
    expect(
      matchesShortcut("Mod+1", key("Digit1", "1", { ctrlKey: true }), true),
    ).toBe(false);
  });

  it("matches Ctrl elsewhere", () => {
    expect(
      matchesShortcut("Mod+1", key("Digit1", "1", { ctrlKey: true }), false),
    ).toBe(true);
  });

  it("requires the exact modifiers", () => {
    const shifted = key("KeyK", "K", { metaKey: true, shiftKey: true });
    expect(matchesShortcut("Mod+K", shifted, true)).toBe(false);
    expect(matchesShortcut("Mod+Shift+K", shifted, true)).toBe(true);
  });

  it("uses the physical key when Shift changes the character", () => {
    const event = key("Comma", "<", { metaKey: true, shiftKey: true });
    expect(matchesShortcut("Mod+Shift+Comma", event, true)).toBe(true);
  });
});
