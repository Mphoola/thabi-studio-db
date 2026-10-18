/**
 * Why a schema came back with no tables.
 *
 * Zero tables is not an explanation. Either the database is empty, or it is
 * not the one that was asked for, or the session is not in it at all — and
 * those need three different fixes, so the question is put to the server
 * rather than guessed at.
 */
import { listDatabases } from "../lib/tauri/commands.js";
import { toUserDetail } from "../lib/errors.js";

export async function whyEmpty(profileId, database) {
  try {
    const names = (await listDatabases(profileId)) ?? [];
    return names.includes(database)
      ? `“${database}” is on this connection, but has no tables visible to it.`
      : `There is no database called “${database}” on this connection. This tab was opened against “${names[0] ?? "nothing"}”.`;
  } catch (failure) {
    return toUserDetail(failure);
  }
}
