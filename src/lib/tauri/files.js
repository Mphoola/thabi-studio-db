import { homeDir, join } from "@tauri-apps/api/path";
import { open } from "@tauri-apps/plugin-dialog";

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
