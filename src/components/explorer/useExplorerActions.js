import { copyText } from "../../lib/clipboard.js";
import { ref } from "vue";
import {
  ClipboardCopy,
  Code,
  Columns3,
  DatabaseZap,
  FileUp,
  FolderPlus,
  HardDrive,
  HardDriveDownload,
  HardDriveUpload,
  Trash2,
  FileCode2,
  FilePlus2,
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
import { sqlTemplate } from "../../lib/sqlTemplates.js";
import { useContainerDialog } from "./useContainerDialog.js";
import { useImportDialog } from "./useImportDialog.js";
import { describeObject, objectDefinition } from "../../lib/tauri/commands.js";

/**
 * What the backup dialog is for, and whether it is open.
 *
 * At module scope, not inside the composable, because the menu that opens it
 * and the panel that renders it call this separately. Refs made per call are
 * two different pieces of state: the menu set one, the dialog watched the
 * other, and nothing on screen happened at all.
 */
const exportTarget = ref(null);
const exportOpen = ref(false);

/** Context-menu items and actions for explorer nodes. */
export function useExplorerActions() {
  const explorer = useExplorer();
  const store = useConnectionProfiles();
  const hostKeys = useHostKeyTrust();
  const workspace = useTabs();
  const profile = store.selectedProfile;
  const toasts = useToasts();
  const containers = useContainerDialog();
  const imports = useImportDialog();
  const writable = () => profile.value?.safeMode !== "read-only";

  /** New and Drop for databases and schemas, where the engine has them and
   *  the connection may change things. */
  function containerItems(node) {
    const driver = profile.value?.driver;
    if (profile.value?.safeMode === "read-only") return [];
    const items = [];
    if (node.type === "database") {
      items.push({
        id: "new-database",
        label: "New Database…",
        icon: DatabaseZap,
      });
      if (driver !== "mysql")
        items.push({
          id: "new-schema",
          label: "New Schema…",
          icon: FolderPlus,
        });
      items.push({
        id: "drop-database",
        label: "Drop Database…",
        icon: Trash2,
        danger: true,
      });
    }
    if (node.type === "schema")
      items.push({
        id: "drop-schema",
        label: "Drop Schema…",
        icon: Trash2,
        danger: true,
      });
    return items.length ? [{ separator: true }, ...items] : [];
  }

  const driverOf = () => profile.value?.driver;

  function menuFor(node) {
    if (!node) return [];
    if (node.type === "routine")
      return [
        { id: "show-definition", label: "Show Definition", icon: Code },
        { separator: true },
        { id: "copy-name", label: "Copy Name", icon: ClipboardCopy },
      ];
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
        { id: "show-create", label: "Show CREATE Statement", icon: Code },
        { separator: true },
        ...(node.kind === "table"
          ? [
              {
                id: "script-select",
                label: "Script as SELECT",
                icon: FilePlus2,
              },
              {
                id: "script-insert",
                label: "Script as INSERT",
                icon: FilePlus2,
              },
              {
                id: "script-update",
                label: "Script as UPDATE",
                icon: FilePlus2,
              },
              {
                id: "script-delete",
                label: "Script as DELETE",
                icon: FilePlus2,
              },
              { separator: true },
            ]
          : []),
        ...(node.kind === "table" && writable()
          ? [{ id: "import-data", label: "Import Data…", icon: FileUp }]
          : []),
        { id: "export-sql", label: "Create Backup…", icon: FileCode2 },
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
            ...(node.type === "schema" || driverOf() === "mysql"
              ? [{ id: "storage", label: "Show Storage", icon: HardDrive }]
              : []),
            ...((node.type === "schema" || driverOf() === "mysql") && writable()
              ? [
                  {
                    id: "import-new",
                    label: "Import into New Table…",
                    icon: FileUp,
                  },
                ]
              : []),
            { id: "export-sql", label: "Create Backup…", icon: FileCode2 },
          ]
        : []),
      ...(node.type === "database"
        ? [
            {
              id: "native-backup",
              label: "Back Up with Native Tools…",
              icon: HardDriveDownload,
            },
            ...(writable()
              ? [
                  {
                    id: "native-restore",
                    label: "Restore from Backup…",
                    icon: HardDriveUpload,
                  },
                ]
              : []),
          ]
        : []),
      ...containerItems(node),
      { separator: true },
      ...copy,
    ].filter((item) => node.type !== "group" || item.id !== "copy-name");
  }

  /**
   * Opens a new query tab with an object's CREATE statement or a template
   * for it. Nothing runs: the tab is a starting point.
   */
  async function openScript(id, node, engine) {
    const profileId = profile.value?.id;
    try {
      const sql =
        id === "show-create"
          ? await objectDefinition(profileId, node)
          : sqlTemplate(
              id.slice("script-".length),
              engine,
              node,
              (await describeObject(profileId, node)).columns,
            );
      workspace.newQuery({
        sql,
        database: node.database ?? null,
        title:
          id === "show-create"
            ? `CREATE ${node.name}`
            : `${id.slice("script-".length).toUpperCase()} ${node.name}`,
      });
    } catch (error) {
      toasts.error(`Couldn’t script “${node.name}”`, toUserDetail(error));
    }
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
      case "show-definition":
        return explorer.open(node);
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
      case "native-backup":
      case "native-restore":
        return imports.showBackup({
          profileId: profile.value?.id,
          database: node.database,
          mode: id === "native-backup" ? "backup" : "restore",
        });
      case "run-dump":
        return imports.showRestore({
          profileId: profile.value?.id,
          database: node.database,
        });
      case "import-data":
        return imports.show({
          profileId: profile.value?.id,
          database: node.database,
          schema: node.schema,
          table: node.name,
        });
      case "import-new":
        return imports.show({
          profileId: profile.value?.id,
          database: node.database,
          schema: node.type === "schema" ? node.schema : null,
        });
      case "storage":
        return workspace.openServer({
          profileId: profile.value?.id,
          view: "storage",
          database: node.database,
          schema: node.type === "schema" ? node.schema : null,
          title: `${profile.value?.name ?? "Server"} server`,
        });
      case "new-database":
        return containers.show("createDatabase", {
          profileId: profile.value?.id,
        });
      case "drop-database":
        return containers.show("dropDatabase", {
          profileId: profile.value?.id,
          name: node.database ?? node.name,
        });
      case "new-schema":
        return containers.show("createSchema", {
          profileId: profile.value?.id,
          database: node.database ?? node.name,
        });
      case "drop-schema":
        return containers.show("dropSchema", {
          profileId: profile.value?.id,
          database: node.database,
          name: node.schema ?? node.name,
        });
      case "show-create":
      case "script-select":
      case "script-insert":
      case "script-update":
      case "script-delete":
        return openScript(id, node, engine);
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
    exportTarget.value = {
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
    exportOpen.value = true;
  }

  return { menuFor, run, target: exportTarget, open: exportOpen };
}
