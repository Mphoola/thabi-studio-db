import { completionSchema } from "../lib/tauri/commands.js";
import { isTauri } from "../lib/tauri/ipc.js";

/**
 * Tables and columns for editor autocomplete, read once per connection and
 * database and kept for the session, so suggestions never wait on the
 * server. Refreshing the explorer drops a connection's cache.
 */
const cache = new Map(); // `${profileId}\0${database}` -> Promise<tables>

export function loadCompletionSchema(profileId, tabId, database) {
  if (!isTauri() || !profileId) return Promise.resolve([]);
  const key = `${profileId}\u0000${database ?? ""}`;
  if (!cache.has(key)) {
    const request = completionSchema(profileId, tabId, database).catch(() => {
      cache.delete(key); // Try again next time rather than keep a failure.
      return [];
    });
    cache.set(key, request);
  }
  return cache.get(key);
}

export function forgetCompletionSchema(profileId) {
  for (const key of [...cache.keys()])
    if (key.startsWith(`${profileId}\u0000`)) cache.delete(key);
}

/**
 * The tables in CodeMirror's SQL schema shape: nested by schema where the
 * engine has schemas, so `public.orders` and plain `orders` both complete.
 */
export function toEditorSchema(tables) {
  const schema = {};
  for (const { schema: owner, table, columns } of tables) {
    if (owner) (schema[owner] ??= {})[table] = columns;
    else schema[table] = columns;
  }
  return schema;
}

/** The schema unqualified names resolve in, per engine. */
export function defaultSchemaFor(engine) {
  if (engine === "postgres") return "public";
  if (engine === "sqlserver") return "dbo";
  return undefined;
}
