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

/** Native picker for a TLS client certificate (PEM or DER). */
export async function pickClientCertificate() {
  const path = await open({
    title: "Choose Client Certificate",
    multiple: false,
    directory: false,
    filters: [
      { name: "Certificates", extensions: ["pem", "crt", "cer", "der"] },
    ],
  });
  return typeof path === "string" ? path : null;
}

/** Native picker for the private key that matches a client certificate. */
export async function pickClientKey() {
  const path = await open({
    title: "Choose Client Private Key",
    multiple: false,
    directory: false,
    filters: [{ name: "Private keys", extensions: ["pem", "key", "der"] }],
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

/** The formats a table or result can be written out as, with extensions. */
export const EXPORT_FORMATS = [
  { value: "csv", label: "CSV", extension: "csv" },
  { value: "tsv", label: "TSV (tab-separated)", extension: "csv" },
  { value: "json", label: "JSON", extension: "json" },
  { value: "markdown", label: "Markdown table", extension: "md" },
  { value: "sqlInsert", label: "SQL INSERT statements", extension: "sql" },
  { value: "sqlUpdate", label: "SQL UPDATE statements", extension: "sql" },
  { value: "xlsx", label: "Excel workbook (.xlsx)", extension: "xlsx" },
];

/**
 * Native save dialog for an export. `format` picks the default extension, so
 * the file the user chooses is the kind they asked for.
 */
export async function pickExportDestination(defaultName, extension = "csv") {
  const path = await save({
    title: "Export",
    defaultPath: `${defaultName.replace(/[\\/:*?"<>|]/g, "_")}.${extension}`,
  });
  return typeof path === "string" ? path : null;
}

/** Reveals a file in Finder, File Explorer or the Linux file manager. */
export function showInFileManager(path) {
  return revealItemInDir(path);
}

/** Save dialog for a connections export (.json). */
export async function pickConnectionsExport() {
  const path = await save({
    title: "Export Connections",
    defaultPath: "thabi-connections.json",
    filters: [{ name: "Connections", extensions: ["json"] }],
  });
  return typeof path === "string" ? path : null;
}

/** Open dialog for a connections export to import. */
export async function pickConnectionsImport() {
  const path = await open({
    title: "Import Connections",
    multiple: false,
    directory: false,
    filters: [{ name: "Connections", extensions: ["json"] }],
  });
  return typeof path === "string" ? path : null;
}

/** Open dialog for a .sql file. */
export async function pickSqlFile(title = "Open SQL File") {
  const path = await open({
    title,
    multiple: false,
    directory: false,
    filters: [{ name: "SQL", extensions: ["sql", "txt"] }],
  });
  return typeof path === "string" ? path : null;
}

/** Native save dialog for the recovery codes text file. */
export async function pickRecoveryCodesDestination() {
  const path = await save({
    title: "Save Recovery Codes",
    defaultPath: "thabi-studio-recovery-codes.txt",
    filters: [{ name: "Text", extensions: ["txt"] }],
  });
  return typeof path === "string" ? path : null;
}

/** Native save dialog for a SQL dump. Resolves to a path or null. */
export async function pickSqlDestination(defaultName) {
  const path = await save({
    title: "Export SQL",
    defaultPath: `${defaultName.replace(/[\\/:*?"<>|]/g, "_")}.sql`,
    filters: [{ name: "SQL", extensions: ["sql"] }],
  });
  return typeof path === "string" ? path : null;
}

/**
 * Where a backup goes when nobody has chosen anywhere else.
 *
 * A folder of its own, named for the app, dated by day: backups written to a
 * Downloads folder get lost among downloads, and backups written into the
 * database's own directory are the first thing a cleanup removes.
 *
 * The caller shows this before anything is written, so the path is never a
 * surprise discovered afterwards.
 */
export async function defaultBackupPath(database, when = new Date()) {
  const day = when.toISOString().slice(0, 10);
  const name = `${String(database).replace(/[\\/:*?"<>|]/g, "_")}-${day}.sql`;
  return join(await homeDir(), "Documents", "Thabi Backups", name);
}
