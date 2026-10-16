import { computed, onBeforeUnmount, shallowRef } from "vue";
import { matchesShortcut } from "../lib/shortcuts.js";

/**
 * @typedef {object} Command
 * @property {string} id         Stable id, e.g. "view.toggleSidebar".
 * @property {string} title      Title Case, as shown in menus and the palette.
 * @property {string} [group]    Palette group, e.g. "View".
 * @property {string} [shortcut] e.g. "Mod+1". See lib/shortcuts.js.
 * @property {() => void} run
 * @property {() => boolean} [enabled]
 */

const commands = shallowRef(/** @type {Map<string, Command>} */ (new Map()));

/** @param {Command} command */
export function registerCommand(command) {
  const next = new Map(commands.value);
  next.set(command.id, command);
  commands.value = next;
  return () => {
    const without = new Map(commands.value);
    without.delete(command.id);
    commands.value = without;
  };
}

/** Registers commands for the lifetime of the calling component. */
export function useCommandRegistration(list) {
  const disposers = list.map(registerCommand);
  onBeforeUnmount(() => disposers.forEach((dispose) => dispose()));
}

let paused = () => false;

/** Stops every command and shortcut while `when()` is true (the app lock). */
export function pauseCommandsWhen(when) {
  paused = when;
}

export function runCommand(id) {
  if (paused()) return;
  const command = commands.value.get(id);
  if (command && (command.enabled?.() ?? true)) command.run();
}

export function useCommands() {
  return {
    commands: computed(() =>
      [...commands.value.values()].filter((c) => c.enabled?.() ?? true),
    ),
    runCommand,
  };
}

/**
 * One global keydown listener dispatches every registered shortcut.
 * Shortcuts without a modifier are ignored while typing in a field.
 */
export function installShortcutListener(target = window) {
  function onKeydown(event) {
    if (event.defaultPrevented || event.isComposing || paused()) return;
    for (const command of commands.value.values()) {
      if (!command.shortcut || !matchesShortcut(command.shortcut, event))
        continue;
      if (!(command.enabled?.() ?? true)) continue;
      event.preventDefault();
      command.run();
      return;
    }
  }
  target.addEventListener("keydown", onKeydown);
  return () => target.removeEventListener("keydown", onKeydown);
}
