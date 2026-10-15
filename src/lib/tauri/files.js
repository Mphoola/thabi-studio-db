import { homeDir, join } from "@tauri-apps/api/path";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

/** Native file picker for a PEM CA certificate. Resolves to a path or null. */
export async function pickCaCertificate() {
  const path = await open({
    title: "Choose CA Certificate",
    multiple: false,
    directory: false,
    filters: [{ name: "Certificates", extensions: ["pem", "crt", "cer"] }],
  });
  return typeof path === "string" ? path : null;
}

/** Native file picker for an SSH private key, starting in ~/.ssh. */
export async function pickSshKey() {
  let defaultPath;
  try {
    defaultPath = await join(await homeDir(), ".ssh");
  } catch {
    defaultPath = undefined;
  }
  const path = await open({
    title: "Choose SSH Private Key",
    multiple: false,
    directory: false,
    defaultPath,
  });
  return typeof path === "string" ? path : null;
}

/** Native save dialog for a CSV export. Resolves to a path or null. */
export async function pickCsvDestination(defaultName) {
  const path = await save({
    title: "Export CSV",
    defaultPath: `${defaultName.replace(/[\\/:*?"<>|]/g, "_")}.csv`,
    filters: [{ name: "CSV", extensions: ["csv"] }],
  });
  return typeof path === "string" ? path : null;
}

/** Reveals a file in Finder, File Explorer or the Linux file manager. */
export function showInFileManager(path) {
  return revealItemInDir(path);
}
