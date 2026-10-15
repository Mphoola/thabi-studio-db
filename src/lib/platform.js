/**
 * The desktop platform the app runs on, and the words people there use for
 * things like the credential store and the file manager.
 */
const agent =
  typeof navigator === "undefined"
    ? ""
    : `${navigator.userAgent} ${navigator.platform ?? ""}`;

/** @type {"mac" | "windows" | "linux"} */
export const platform = /Mac|iPhone|iPad/i.test(agent)
  ? "mac"
  : /Win/i.test(agent)
    ? "windows"
    : "linux";

export const isMac = platform === "mac";

const WORDS = {
  mac: {
    credentialStore: "Keychain",
    fileManager: "Show in Finder",
    computer: "this Mac",
  },
  windows: {
    credentialStore: "Windows Credential Manager",
    fileManager: "Show in File Explorer",
    computer: "this computer",
  },
  linux: {
    credentialStore: "your system keyring",
    fileManager: "Show in Folder",
    computer: "this computer",
  },
};

export const words = WORDS[platform];
