import { useCommandRegistration } from "./useCommands.js";
import { useRowDetail } from "./useRowDetail.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useHostKeyTrust } from "./useHostKeyTrust.js";
import { useDialogs } from "./useDialogs.js";
import { useAppLock } from "./useAppLock.js";
import { useExplorer } from "./useExplorer.js";
import { useTabs } from "./useTabs.js";
import { useQuerySave } from "./useQuerySave.js";
import { useLayout } from "./useLayout.js";
import { useTheme } from "./useTheme.js";
import { useToasts } from "./useToasts.js";
import { toUserDetail } from "../lib/errors.js";
import { useConnectionTransfer } from "./useConnectionTransfer.js";
import { useSqlFiles } from "./useSqlFiles.js";
import { openWorkspaceWindow } from "../lib/tauri/commands.js";

/**
 * App-wide commands. Shortcuts follow platform conventions (⌘ on macOS, Ctrl elsewhere) and are reserved
 * in docs/design/workspace.md so later phases don't collide.
 */
export function useAppCommands() {
  const dialogs = useDialogs();
  const appLock = useAppLock();
  const connections = useConnectionProfiles();
  const layout = useLayout();
  const explorer = useExplorer();
  const workspace = useTabs();
  const queryTabActive = () => workspace.active.value?.type === "query";
  const rowDetail = useRowDetail();
  const saver = useQuerySave();
  const theme = useTheme();
  const toasts = useToasts();
  const hostKeys = useHostKeyTrust();
  const transfer = useConnectionTransfer();
  const sqlFiles = useSqlFiles();

  async function connectSelected() {
    const profile = connections.selectedProfile.value;
    if (!profile) return;
    try {
      await hostKeys.run(() => connections.connect(profile.id));
    } catch (error) {
      toasts.error(
        `Couldn’t connect to “${profile.name}”`,
        toUserDetail(error),
      );
    }
  }

  async function disconnectSelected() {
    const profile = connections.selectedProfile.value;
    if (!profile) return;
    try {
      await connections.disconnect(profile.id);
    } catch (error) {
      toasts.error(
        `Couldn’t disconnect from “${profile.name}”`,
        toUserDetail(error),
      );
    }
  }

  const commands = [
    {
      id: "connections.new",
      title: "New Connection…",
      group: "Connections",
      shortcut: "Mod+N",
      enabled: () => connections.nativeAvailable,
      run: connections.openNew,
    },
    {
      id: "connections.connectSelected",
      title: "Connect Selected Connection",
      group: "Connections",
      enabled: () =>
        connections.nativeAvailable &&
        Boolean(connections.selectedProfile.value) &&
        !connections.isConnected(connections.selectedProfile.value.id),
      run: connectSelected,
    },
    {
      id: "connections.disconnectSelected",
      title: "Disconnect Selected Connection",
      group: "Connections",
      enabled: () =>
        connections.nativeAvailable &&
        Boolean(connections.selectedProfile.value) &&
        connections.isConnected(connections.selectedProfile.value.id),
      run: disconnectSelected,
    },
    {
      id: "app.commandPalette",
      title: "Open Anything",
      group: "General",
      shortcut: "Mod+P",
      run: () => dialogs.show("palette"),
    },
    {
      id: "app.settings",
      title: "Settings…",
      group: "General",
      shortcut: "Mod+Comma",
      run: () => layout.openSettings(),
    },
    {
      id: "app.lock",
      title: "Lock Thabi Studio",
      group: "General",
      enabled: () => appLock.state.value.enabled,
      run: () => appLock.lock(),
    },
    {
      id: "app.shortcuts",
      title: "Keyboard Shortcuts",
      group: "Help",
      shortcut: "Mod+Slash",
      run: () => dialogs.show("shortcuts"),
    },
    {
      id: "app.about",
      title: "About Thabi Studio",
      group: "Help",
      run: () => dialogs.show("about"),
    },
    {
      id: "connections.switch",
      title: "Switch Connection…",
      group: "Connections",
      shortcut: "Mod+Shift+K",
      enabled: () => connections.profiles.value.length > 0,
      run: () => dialogs.show("switcher"),
    },
    {
      id: "app.newWindow",
      title: "New Window",
      group: "General",
      shortcut: "Mod+Shift+N",
      run: () =>
        openWorkspaceWindow().catch((error) =>
          toasts.error("Couldn’t open a window", toUserDetail(error)),
        ),
    },
    {
      id: "connections.import",
      title: "Import Connections…",
      group: "Connections",
      run: () => transfer.importFile(),
    },
    {
      id: "connections.export",
      title: "Export Connections…",
      group: "Connections",
      enabled: () => connections.profiles.value.length > 0,
      run: () => transfer.exportAll(),
    },
    {
      id: "tabs.newQuery",
      title: "New Query Tab",
      group: "Query",
      shortcut: "Mod+T",
      run: () => workspace.newQuery(),
    },
    {
      id: "tabs.close",
      title: "Close Tab",
      group: "Query",
      shortcut: "Mod+W",
      enabled: () => Boolean(workspace.active.value),
      run: () => workspace.close(),
    },
    {
      id: "grid.rowDetail",
      title: "Show Row Detail",
      group: "Query",
      shortcut: "Mod+Alt+I",
      // `enabled` is a predicate the registry calls, so unwrap the ref.
      enabled: () => Boolean(rowDetail.available.value),
      run: () => rowDetail.open(),
    },
    {
      id: "query.save",
      title: "Save Query",
      group: "Query",
      shortcut: "Mod+S",
      enabled: queryTabActive,
      run: () => saver.saveActive(),
    },
    {
      id: "query.focusEditor",
      title: "Focus Editor",
      group: "Query",
      shortcut: "Mod+E",
      enabled: queryTabActive,
      run: () => workspace.activeHandle()?.focus(),
    },
    {
      id: "query.run",
      title: "Run Statement",
      group: "Query",
      shortcut: "Mod+Enter",
      enabled: queryTabActive,
      run: () => workspace.activeHandle()?.run(false),
    },
    {
      id: "query.format",
      title: "Format SQL",
      group: "Query",
      shortcut: "Mod+I",
      enabled: queryTabActive,
      run: () => workspace.activeHandle()?.format(),
    },
    {
      id: "query.explain",
      title: "Explain Query",
      group: "Query",
      shortcut: "Mod+Shift+E",
      enabled: queryTabActive,
      run: () => workspace.activeHandle()?.explain(),
    },
    {
      id: "query.explainAnalyze",
      title: "Explain Analyze (PostgreSQL)…",
      group: "Query",
      enabled: () =>
        queryTabActive() &&
        connections.profiles.value.find(
          (p) => p.id === workspace.active.value?.profileId,
        )?.driver === "postgres",
      run: () => workspace.activeHandle()?.explainAnalyze(),
    },
    {
      id: "query.openFile",
      title: "Open SQL File…",
      group: "Query",
      shortcut: "Mod+O",
      run: () => sqlFiles.openFile(),
    },
    {
      id: "query.runFile",
      title: "Run SQL File…",
      group: "Query",
      enabled: () => connections.profiles.value.length > 0,
      run: () => sqlFiles.runFile(),
    },
    {
      id: "query.split",
      title: "Split Editor",
      group: "Query",
      shortcut: "Mod+Shift+D",
      enabled: queryTabActive,
      run: () => workspace.toggleSplit(),
    },
    {
      id: "query.minify",
      title: "Minify SQL",
      group: "Query",
      enabled: queryTabActive,
      run: () => workspace.activeHandle()?.minify(),
    },
    {
      id: "query.runAll",
      title: "Run All",
      group: "Query",
      shortcut: "Mod+Shift+Enter",
      enabled: queryTabActive,
      run: () => workspace.activeHandle()?.run(true),
    },
    {
      id: "query.cancel",
      title: "Cancel Query",
      group: "Query",
      shortcut: "Mod+Period",
      enabled: () => Boolean(workspace.active.value?.running),
      run: () => workspace.activeHandle()?.cancel(),
    },
    {
      id: "view.workspace",
      title: "Show Query Tabs",
      group: "View",
      run: () => (layout.mainView.value = "workspace"),
    },
    {
      id: "explorer.refresh",
      title: "Refresh Explorer",
      group: "View",
      shortcut: "Mod+R",
      enabled: () => explorer.connected.value,
      run: () => {
        layout.sidebarView.value = "explorer";
        layout.sidebarOpen.value = true;
        explorer.refresh();
      },
    },
    {
      // A feature only reachable from a context menu is a feature nobody
      // finds. The explorer keeps the menu item; this is how it is looked up.
      id: "view.diagram",
      title: "Show Schema Diagram",
      group: "View",
      shortcut: "Mod+Shift+G",
      enabled: () => explorer.connected.value,
      run: () => {
        const status = connections.statusFor(explorer.profile.value?.id);
        if (!status?.connectionId) {
          toasts.error("Nothing to draw", "Connect to a database first.");
          return;
        }
        layout.sidebarView.value = "explorer";
        layout.sidebarOpen.value = true;
        workspace.openDiagram({
          profileId: explorer.profile.value?.id,
          connectionId: status.connectionId,
          database: status.server?.database,
          schema: null,
          title: `${status.server?.database ?? "Schema"} diagram`,
        });
      },
    },
    {
      id: "view.connections",
      title: "Show Connections",
      group: "View",
      run: () => (layout.mainView.value = "welcome"),
    },
    {
      id: "view.toggleSidebar",
      title: "Toggle Sidebar",
      group: "View",
      shortcut: "Mod+1",
      run: layout.toggleSidebar,
    },
    ...[
      ["explorer", "Show Explorer"],
      ["saved", "Show Saved Queries"],
      ["history", "Show Query History"],
    ].map(([view, title]) => ({
      id: `view.sidebar.${view}`,
      title,
      group: "View",
      run: () => {
        layout.sidebarView.value = view;
        layout.sidebarOpen.value = true;
      },
    })),
    ...[
      ["system", "Use System Appearance"],
      ["light", "Use Light Appearance"],
      ["dark", "Use Dark Appearance"],
    ].map(([mode, title]) => ({
      id: `view.appearance.${mode}`,
      title,
      group: "Appearance",
      run: () => theme.setMode(mode),
    })),
  ];

  useCommandRegistration(commands);
}
