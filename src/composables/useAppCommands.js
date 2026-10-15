import { useCommandRegistration } from "./useCommands.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useHostKeyTrust } from "./useHostKeyTrust.js";
import { useDialogs } from "./useDialogs.js";
import { useExplorer } from "./useExplorer.js";
import { useTabs } from "./useTabs.js";
import { useQuerySave } from "./useQuerySave.js";
import { useLayout } from "./useLayout.js";
import { useTheme } from "./useTheme.js";
import { useToasts } from "./useToasts.js";
import { toUserDetail } from "../lib/errors.js";

/**
 * App-wide commands. Shortcuts follow platform conventions (⌘ on macOS, Ctrl elsewhere) and are reserved
 * in docs/design/workspace.md so later phases don't collide.
 */
export function useAppCommands() {
  const dialogs = useDialogs();
  const connections = useConnectionProfiles();
  const layout = useLayout();
  const explorer = useExplorer();
  const workspace = useTabs();
  const queryTabActive = () => workspace.active.value?.type === "query";
  const saver = useQuerySave();
  const theme = useTheme();
  const toasts = useToasts();
  const hostKeys = useHostKeyTrust();

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
      run: () => dialogs.show("settings"),
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

  if (import.meta.env.DEV) {
    commands.push(
      {
        id: "dev.gallery",
        title: "Show Component Gallery",
        group: "Developer",
        run: () => (layout.mainView.value = "gallery"),
      },
      {
        id: "dev.welcome",
        title: "Show Welcome View",
        group: "Developer",
        run: () => (layout.mainView.value = "welcome"),
      },
    );
  }

  useCommandRegistration(commands);
}
