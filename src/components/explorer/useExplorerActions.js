import { copyText } from "../../lib/clipboard.js";
import {
  ClipboardCopy,
  Columns3,
  RefreshCw,
  Repeat,
  Rows3,
} from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useExplorer } from "../../composables/useExplorer.js";
import { useHostKeyTrust } from "../../composables/useHostKeyTrust.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { qualifiedName } from "../../lib/explorerTree.js";

/** Context-menu items and actions for explorer nodes. */
export function useExplorerActions() {
  const explorer = useExplorer();
  const store = useConnectionProfiles();
  const hostKeys = useHostKeyTrust();
  const toasts = useToasts();

  function menuFor(node) {
    if (!node) return [];
    const copy = [
      { id: "copy-name", label: "Copy Name", icon: ClipboardCopy },
      ...(node.type === "object"
        ? [{ id: "copy-qualified", label: "Copy Qualified Name" }]
        : []),
    ];
    if (node.type === "object")
      return [
        { id: "open-data", label: "Open Data", icon: Rows3 },
        { id: "open-structure", label: "Show Structure", icon: Columns3 },
        { separator: true },
        ...copy,
      ];
    const current = explorer.status.value?.server?.database;
    const canSwitch =
      node.type === "database" &&
      store.selectedProfile.value?.driver === "postgres" &&
      node.database !== current;
    return [
      ...(canSwitch
        ? [{ id: "switch", label: "Switch to This Database", icon: Repeat }]
        : []),
      { id: "refresh", label: "Refresh", icon: RefreshCw, shortcut: "Mod+R" },
      { separator: true },
      ...copy,
    ].filter((item) => node.type !== "group" || item.id !== "copy-name");
  }

  async function copy(text) {
    try {
      await copyText(text);
      toasts.info("Copied", text);
    } catch {
      toasts.error("Couldn’t copy", "The clipboard isn’t available.");
    }
  }

  async function run(id, node) {
    const engine = store.selectedProfile.value?.driver;
    const profile = store.selectedProfile.value;
    switch (id) {
      case "open-data":
        return explorer.open(node, "data");
      case "open-structure":
        return explorer.open(node, "structure");
      case "copy-name":
        return copy(node.name ?? node.label);
      case "copy-qualified":
        return copy(qualifiedName(engine, node));
      case "refresh":
        return explorer.refresh(node);
      case "switch":
        try {
          await hostKeys.run(() => store.connect(profile.id, node.database));
        } catch (error) {
          toasts.error(
            `Couldn’t switch to “${node.database}”`,
            toUserDetail(error),
          );
        }
        return;
    }
  }

  return { menuFor, run };
}
