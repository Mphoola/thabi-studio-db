import { useCommandRegistration } from "./useCommands.js";
import { useConnectionProfiles } from "./useConnectionProfiles.js";
import { useDialogs } from "./useDialogs.js";
import { useLayout } from "./useLayout.js";
import { useTheme } from "./useTheme.js";
import { useToasts } from "./useToasts.js";
import { toUserMessage } from "../lib/errors.js";

/**
 * App-wide commands. Shortcuts follow macOS conventions and are reserved
 * in docs/design/workspace.md so later phases don't collide.
 */
export function useAppCommands() {
  const dialogs = useDialogs();
  const connections = useConnectionProfiles();
  const layout = useLayout();
  const theme = useTheme();
  const toasts = useToasts();

  async function connectSelected() {
    const profile = connections.selectedProfile.value;
    if (!profile) return;
    try {
      await connections.connect(profile.id);
      toasts.success(`Connected to ${profile.name}`);
    } catch (error) {
      toasts.error("Couldn't connect to database", toUserMessage(error));
    }
  }

  async function disconnectSelected() {
    const profile = connections.selectedProfile.value;
    if (!profile) return;
    try {
      await connections.disconnect(profile.id);
      toasts.success(`Disconnected from ${profile.name}`);
    } catch (error) {
      toasts.error("Couldn't disconnect from database", toUserMessage(error));
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
