/**
 * Development-only fake backend for checking the UI in a plain browser
 * (open the Vite URL with ?mock). It answers the Tauri commands with sample
 * data so screens can be reviewed without the native app or databases.
 * Never bundled in release builds (see main.js).
 */
import { mockIPC } from "@tauri-apps/api/mocks";

const profile = {
  id: "sample",
  name: "Sample (mock)",
  environment: "staging",
  driver: "postgres",
  host: "db.internal",
  port: 5432,
  databaseName: "shop",
  username: "app",
  authMode: "password",
  tlsMode: "verify-full",
  tlsCaPath: "",
  safeMode: "off",
  sshEnabled: true,
  sshHost: "bastion.example.com",
  sshPort: 22,
  sshUsername: "deploy",
  sshAuthMode: "agent",
  sshKeyPath: "",
  hasDatabasePassword: true,
  hasSshPassword: false,
  hasSshKeyPassphrase: false,
  createdAt: "",
  updatedAt: "",
  lastConnectedAt: null,
};

let connected = false;
const status = () => ({
  profileId: "sample",
  connectionId: "sample:1",
  state: "connected",
  server: { engine: "postgres", version: "PostgreSQL 16.4", database: "shop" },
  capabilities: null,
});

const v = (value) =>
  value === null ? { kind: "null" } : { kind: "value", value };

// A large sample result kept "in Rust" for paging, sorting and copying.
const bigRows = Array.from({ length: 5000 }, (_, i) => [
  v(String(100000 + i)),
  i % 7 === 0 ? v(null) : v(`customer${i}@example.com`),
  v(((i * 37.5) % 9000).toFixed(2)),
  i % 11 === 0 ? v("") : v(["pending", "paid", "refunded"][i % 3]),
  v(JSON.stringify({ items: i % 5, gift: i % 2 === 0 })),
]);
const bigColumns = [
  { name: "id", typeName: "INT8" },
  { name: "customer_email", typeName: "TEXT" },
  { name: "total", typeName: "NUMERIC" },
  { name: "status", typeName: "TEXT" },
  { name: "meta", typeName: "JSONB" },
];

const handlers = {
  app_info: () => ({ name: "Thabi Studio", version: "1.0.0-mock" }),
  get_app_settings: () => ({
    themeMode: "system",
    sidebarWidth: 260,
    sidebarOpen: true,
    selectedProfileId: "sample",
  }),
  import_legacy_preferences: () => handlers.get_app_settings(),
  set_app_setting: () => null,
  list_connection_profiles: () => [
    profile,
    {
      ...profile,
      id: "prod",
      name: "Orders (production)",
      environment: "production",
      safeMode: "confirm-writes",
      driver: "sqlserver",
      host: "sql-prod.internal",
      port: 1433,
      databaseName: "",
      sshEnabled: false,
      lastConnectedAt: "2026-10-14 08:00:00",
    },
    {
      ...profile,
      id: "local",
      name: "Local MySQL",
      environment: "local",
      driver: "mysql",
      host: "127.0.0.1",
      port: 3306,
      databaseName: "app",
      tlsMode: "disable",
      sshEnabled: false,
      lastConnectedAt: new Date(Date.now() - 2 * 3600e3).toISOString(),
    },
  ],
  list_connections: () => (connected ? [status()] : []),
  connect_connection_profile: () => {
    connected = true;
    return status();
  },
  disconnect_connection_profile: () => {
    connected = false;
    return null;
  },
  select_connection_profile: () => null,
  list_databases: () => ["analytics", "shop", "postgres"],
  list_schemas: ({ database }) => {
    if (database !== "shop")
      throw {
        kind: "database_not_open",
        message: `This connection is using “shop”. Switch to “${database}” to browse it.`,
      };
    return ["public", "Sales Ünïcödé", "empty"];
  },
  list_objects: ({ schema }) =>
    schema === "empty"
      ? []
      : [
          { name: "orders", kind: "table" },
          { name: "Order Details", kind: "table" },
          { name: "customers", kind: "table" },
          { name: "Big Orders", kind: "view" },
          { name: "daily_totals", kind: "materialized_view" },
        ],
  describe_object: ({ kind }) => ({
    columns: [
      {
        name: "id",
        dataType: "bigint",
        nullable: false,
        default: null,
        primaryKey: 1,
      },
      {
        name: "customer_email",
        dataType: "text",
        nullable: true,
        default: null,
        primaryKey: null,
        references: { schema: "public", table: "customers", column: "email" },
      },
      {
        name: "total",
        dataType: "numeric(12,2)",
        nullable: false,
        default: "0",
        primaryKey: null,
      },
      {
        name: "status",
        dataType: "order_status",
        nullable: false,
        default: "'pending'::order_status",
        primaryKey: null,
        enumValues: ["pending", "paid", "refunded"],
      },
    ],
    indexes:
      kind === "view"
        ? null
        : [
            {
              name: "orders_pkey",
              columns: ["id"],
              unique: true,
              primary: true,
            },
            {
              name: "orders_status_idx",
              columns: ["status"],
              unique: false,
              primary: false,
            },
          ],
  }),
};

let cancelRun = null;
handlers.run_statement = ({ sql }) =>
  new Promise((resolve, reject) => {
    const text = sql.toLowerCase();
    if (text.includes("missing_table"))
      return reject({
        kind: "server",
        message: "The database returned an error.",
        detail: 'relation "missing_table" does not exist',
        code: "42P01",
      });
    const finish = () => {
      cancelRun = null;
      if (/^\s*(update|delete|insert)/.test(text))
        return resolve({
          columns: [],
          rows: [],
          rowsAffected: 3,
          truncated: false,
          extraResultSets: 0,
          elapsedMs: 18,
        });
      resolve({
        columns: bigColumns,
        rows: bigRows.slice(0, 200),
        totalRows: bigRows.length,
        resultId: "r1",
        rowsAffected: null,
        truncated: false,
        extraResultSets: 0,
        elapsedMs: 9,
      });
    };
    if (!text.includes("sleep")) return finish();
    const timer = setTimeout(finish, 4000);
    cancelRun = () => {
      clearTimeout(timer);
      cancelRun = null;
      reject({
        kind: "query_cancelled",
        message: "Query cancelled.",
        data: { sessionReset: false },
      });
    };
  });
function sortedRows(sort) {
  if (!sort) return bigRows;
  const numeric =
    bigColumns.find((column) => column.name === sort.column)?.typeName !==
    "TEXT";
  const at = bigColumns.findIndex((column) => column.name === sort.column);
  const key = (row) => row[at].value;
  return [...bigRows].sort((a, b) => {
    const x = key(a),
      y = key(b);
    if (x == null) return 1;
    if (y == null) return -1;
    const c = numeric ? Number(x) - Number(y) : x.localeCompare(y);
    return sort.descending ? -c : c;
  });
}
handlers.fetch_rows = ({ offset, limit, sort }) =>
  sortedRows(sort).slice(offset, offset + limit);
handlers.copy_rows = ({ scope }) => {
  const [start, end] = scope.rows ?? [0, bigRows.length];
  return bigRows
    .slice(start, end)
    .map((r) => r.map((c) => c.value ?? "").join("\t"))
    .join("\n");
};
handlers.export_csv = ({ path }) => ({ rows: bigRows.length, path });
handlers.export_result = ({ path }) => ({ rows: bigRows.length, path });

handlers.open_object_data = ({ name }) =>
  name === "customers"
    ? Promise.reject({
        kind: "permission_denied",
        message: "This user doesn't have permission to read that.",
        detail: "permission denied for table customers",
        code: "42501",
      })
    : handlers.run_statement({ sql: "select" });

// Staged row edits: shows plausible SQL; nothing is stored.
handlers.preview_row_changes = ({ target, changes }) =>
  changes.map((c) => {
    const where = (c.key ?? [])
      .map((k) => `"${k.column}" = '${k.value}'`)
      .join(" AND ");
    if (c.kind === "delete")
      return `DELETE FROM "${target.name}" WHERE ${where}`;
    if (c.kind === "insert")
      return `INSERT INTO "${target.name}" (${c.values.map((v) => `"${v.column}"`).join(", ")}) VALUES (…)`;
    const set = c.set.map((v) => `"${v.column}" = '${v.value}'`).join(", ");
    return `UPDATE "${target.name}" SET ${set} WHERE ${where}`;
  });
handlers.lookup_reference = ({ search }) =>
  Array.from({ length: 40 }, (_, i) => ({
    value: `customer${i}@example.com`,
    label: `Customer ${i}`,
  })).filter((r) => r.value.includes(search.trim().toLowerCase()));
handlers.completion_schema = () => [
  {
    schema: "public",
    table: "orders",
    columns: ["id", "customer_email", "total", "status"],
  },
  { schema: "public", table: "customers", columns: ["id", "email", "name"] },
];
handlers.preview_filter = ({ filters }) =>
  filters.length
    ? `WHERE ${filters.map((f) => `"${f.column}" ${f.op} '${f.value}'`).join(" AND ")}`
    : "";
handlers.list_referencing = () => [];
// Saving a connection in the preview: echoes it back as Rust would.
let mockProfileId = 100;
const mockProfile = (input, id) => ({
  ...input,
  id,
  port: Number(input.port),
  sshPort: Number(input.sshPort || 22),
  queryTimeoutSeconds: Number(input.queryTimeoutSeconds) || 0,
  maxResultRows: Number(input.maxResultRows) || 0,
  hasDatabasePassword: Boolean(input.databasePassword),
  hasSshPassword: Boolean(input.sshPassword),
  hasSshKeyPassphrase: Boolean(input.sshKeyPassphrase),
  sortOrder: 0,
  lastConnectedAt: null,
});
// The app lock. ?mock&locked starts locked with the password "Preview Pass1",
// the recovery code "11111-22222" and every security answer "preview".
const startLocked = new URLSearchParams(location.search).has("locked");
const mockLock = {
  password: startLocked ? "Preview Pass1" : null,
  locked: startLocked,
  codes: startLocked ? ["11111-22222", "33333-44444"] : [],
  questions: startLocked
    ? [
        { question: "What city were you born in?", answer: "preview" },
        { question: "What was the name of your first pet?", answer: "preview" },
        { question: "What is your favourite book?", answer: "preview" },
      ]
    : [],
};
const mockCodes = () =>
  Array.from({ length: 10 }, () =>
    Array.from({ length: 2 }, () =>
      Math.floor(Math.random() * 0xfffff)
        .toString(16)
        .padStart(5, "0"),
    ).join("-"),
  );
const recoveryOptions = () => ({
  codesLeft: mockLock.codes.length,
  questions: mockLock.questions.map((q) => q.question),
});
const fieldError = (field, message) => ({
  kind: "validation",
  message: "Check the highlighted fields.",
  fields: { [field]: message },
});
const lockStatus = () => ({
  enabled: Boolean(mockLock.password),
  locked: mockLock.locked,
  retryAfterSeconds: 0,
});
const wrongPassword = (field) => ({
  kind: "validation",
  message: "Check the highlighted fields.",
  fields: { [field]: "That password isn't right." },
});
handlers.app_lock_status = lockStatus;
handlers.lock_app = () => {
  mockLock.locked = Boolean(mockLock.password);
  return lockStatus();
};
handlers.unlock_app = ({ password }) => {
  if (password !== mockLock.password) throw wrongPassword("password");
  mockLock.locked = false;
  return lockStatus();
};
handlers.set_app_password = ({ current, password }) => {
  const first = !mockLock.password;
  if (!first && current !== mockLock.password) throw wrongPassword("current");
  mockLock.password = password;
  mockLock.locked = false;
  if (first) mockLock.codes = mockCodes();
  return {
    status: lockStatus(),
    recoveryCodes: first ? [...mockLock.codes] : null,
  };
};
handlers.recovery_options = recoveryOptions;
handlers.recover_with_code = ({ code, password }) => {
  const index = mockLock.codes.indexOf(code.trim().toLowerCase());
  if (index < 0)
    throw fieldError("code", "That code isn't right, or it was already used.");
  mockLock.codes.splice(index, 1);
  mockLock.password = password;
  mockLock.locked = false;
  return lockStatus();
};
handlers.recover_with_answers = ({ answers, password }) => {
  const right = mockLock.questions.every(
    (q, i) => (answers[i] ?? "").trim().toLowerCase() === q.answer,
  );
  if (!right) throw fieldError("answers", "One or more answers aren't right.");
  mockLock.password = password;
  mockLock.locked = false;
  return lockStatus();
};
handlers.regenerate_recovery_codes = ({ current }) => {
  if (current !== mockLock.password) throw wrongPassword("current");
  mockLock.codes = mockCodes();
  return [...mockLock.codes];
};
handlers.set_security_questions = ({ current, questions }) => {
  if (current !== mockLock.password) throw wrongPassword("current");
  mockLock.questions = questions.map((q) => ({
    question: q.question,
    answer: q.answer.trim().toLowerCase(),
  }));
  return recoveryOptions();
};
handlers.remove_security_questions = ({ current }) => {
  if (current !== mockLock.password) throw wrongPassword("current");
  mockLock.questions = [];
  return recoveryOptions();
};
handlers.remove_app_password = ({ current }) => {
  if (current !== mockLock.password) throw wrongPassword("current");
  mockLock.password = null;
  mockLock.codes = [];
  mockLock.questions = [];
  return lockStatus();
};
handlers.reset_app_password = () => {
  mockLock.password = null;
  mockLock.codes = [];
  mockLock.questions = [];
  mockLock.locked = false;
  return lockStatus();
};

handlers.create_connection_profile = ({ input }) => ({
  profile: mockProfile(input, `p${(mockProfileId += 1)}`),
  warning: null,
});
handlers.update_connection_profile = ({ id, input }) => ({
  profile: mockProfile(input, id),
  warning: null,
});
handlers.apply_row_changes = ({ changes }) => changes.length;

handlers.cancel_statement = () => {
  if (!cancelRun) return false;
  cancelRun();
  return true;
};
handlers.close_query_tab = () => null;

// History and saved queries, kept in memory for the session.
const historyStore = [];
const savedStore = [];
const originalRun = handlers.run_statement;
handlers.run_statement = async (args) => {
  const started = Date.now();
  const entry = {
    id: historyStore.length + 1,
    profileId: "sample",
    connectionName: "Sample (mock)",
    databaseName: args.database ?? "shop",
    queryText: args.sql,
    startedAt: new Date().toISOString(),
    durationMs: 0,
    status: "completed",
    rowCount: null,
    errorSummary: null,
  };
  try {
    const result = await originalRun(args);
    entry.rowCount = result.totalRows ?? result.rowsAffected ?? null;
    return result;
  } catch (error) {
    entry.status = error.kind === "query_cancelled" ? "cancelled" : "failed";
    entry.errorSummary = error.detail ?? error.message;
    throw error;
  } finally {
    entry.durationMs = Date.now() - started;
    historyStore.unshift(entry);
  }
};
handlers.list_history = ({ search }) =>
  historyStore.filter((h) =>
    h.queryText.toLowerCase().includes((search ?? "").toLowerCase()),
  );
handlers.delete_history_entry = ({ id }) => {
  historyStore.splice(
    historyStore.findIndex((h) => h.id === id),
    1,
  );
};
handlers.clear_history = () => historyStore.splice(0).length;
handlers.list_saved_queries = () => savedStore.map((q) => ({ ...q }));
handlers.save_query = ({ input }) => {
  if (!input.name.trim())
    throw {
      kind: "validation",
      message: "Check the highlighted fields.",
      fields: { name: "Enter a name of up to 120 characters." },
    };
  let saved = savedStore.find((q) => q.id === input.id);
  if (!saved) {
    saved = { id: `s${savedStore.length + 1}`, createdAt: "", updatedAt: "" };
    savedStore.push(saved);
  }
  Object.assign(saved, {
    name: input.name.trim(),
    queryText: input.queryText,
    profileId: input.profileId ?? null,
    notes: input.notes ?? "",
    folder: (input.folder ?? "").trim(),
    keyword: (input.keyword ?? "").trim(),
  });
  return saved;
};
handlers.rename_saved_folder = ({ from, to }) => {
  const moved = savedStore.filter((q) => q.folder === from);
  for (const q of moved) q.folder = to.trim();
  return moved.length;
};
handlers.delete_saved_query = ({ id }) => {
  savedStore.splice(
    savedStore.findIndex((q) => q.id === id),
    1,
  );
};
handlers.save_workspace_tabs = () => null;
handlers.load_workspace_tabs = () => [];

export function installMockBackend() {
  mockIPC(
    (command, args) => {
      const handler = handlers[command];
      if (!handler)
        throw { kind: "mock", message: `Mock has no "${command}".` };
      return handler(args);
    },
    { shouldMockEvents: true },
  );
}
