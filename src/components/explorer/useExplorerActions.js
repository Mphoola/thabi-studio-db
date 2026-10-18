import { copyText } from "../../lib/clipboard.js";
import { ref } from "vue";
import {
  ClipboardCopy,
  Columns3,
  FileCode2,
  Network,
  RefreshCw,
  Repeat,
  Rows3,
} from "lucide-vue-next";
import { useConnectionProfiles } from "../../composables/useConnectionProfiles.js";
import { useExplorer } from "../../composables/useExplorer.js";
import { useTabs } from "../../composables/useTabs.js";
import { useHostKeyTrust } from "../../composables/useHostKeyTrust.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { diagramTargetFor, qualifiedName } from "../../lib/explorerTree.js";

/** Context-menu items and actions for explorer nodes. */
export function useExplorerActions() {
  const explorer = useExplorer();
  const store = useConnectionProfiles();
  const hostKeys = useHostKeyTrust();
  const workspace = useTabs();
  const profile = store.selectedProfile;
  // What the export dialog is for, and whether it is open. Refs rather than
  // arguments so a menu item can open a dialog it does not render.
  const target = ref(null);
  const open = ref(false);
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
        { id: "export-sql", label: "Export as SQL…", icon: FileCode2 },
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
      // A diagram needs somewhere to draw tables. On MySQL the database *is*
      // the schema, which is why this is offered on both rather than on
      // whichever one happens to be called a schema.
      ...(node.type === "schema" || node.type === "database"
        ? [
            { id: "diagram", label: "Show Diagram", icon: Network },
            { id: "export-sql", label: "Export as SQL…", icon: FileCode2 },
          ]
        : []),
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
    const engine = profile.value?.driver;
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
      case "export-sql":
        // A table exports just itself; a database or schema exports all of it.
        return emitExport(node);
      case "diagram": {
        if (!store.isConnected(profile.value?.id)) {
          toasts.error("Nothing to draw", "Connect to a database first.");
          return;
        }
        const target = diagramTargetFor(node);
        if (!target) {
          toasts.error("Nothing to draw", "Pick a database or a schema first.");
          return;
        }
        return workspace.openDiagram({
          profileId: profile.value?.id,
          ...target,
        });
      }
      case "switch":
        try {
          await hostKeys.run(() =>
            store.connect(profile.value.id, node.database),
          );
        } catch (error) {
          toasts.error(
            `Couldn’t switch to “${node.database}”`,
            toUserDetail(error),
          );
        }
        return;
    }
  }

  /**
   * Where the export dialog opens, for one table or for a whole schema.
   *
   * The dialog needs the connection's profile id, which is what commands are
   * keyed by, and the driver so it can say whether the structure in the file
   * is the server's own words or a rebuild.
   */
  function emitExport(node) {
    const live = store.statusFor(profile.value?.id);
    if (!live?.connectionId) {
      toasts.error("Nothing to export", "Connect to a database first.");
      return;
    }
    const one = node.type === "object";
    target.value = {
      profileId: profile.value.id,
      database: one
        ? node.database
        : (node.database ?? live.server?.database ?? node.name),
      schema: one
        ? (node.schema ?? null)
        : node.type === "schema"
          ? node.name
          : (node.schema ?? null),
      engine: profile.value?.driver ?? "",
      tables: one ? [node.name] : [],
      available: [],
    };
    open.value = true;
  }

  return { menuFor, run, target, open };
}
