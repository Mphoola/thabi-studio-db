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

const handlers = {
  app_info: () => ({ name: "Thabi Studio", version: "0.1.0-mock" }),
  get_app_settings: () => ({
    themeMode: "system",
    sidebarWidth: 260,
    sidebarOpen: true,
    selectedProfileId: "sample",
  }),
  import_legacy_preferences: () => handlers.get_app_settings(),
  set_app_setting: () => null,
  list_connection_profiles: () => [profile],
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
        dataType: "text",
        nullable: false,
        default: "'pending'::text",
        primaryKey: null,
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
  preview_object: ({ name }) => {
    if (name === "customers")
      throw {
        kind: "permission_denied",
        message: "This user doesn't have permission to read that.",
        detail: "permission denied for table customers",
        code: "42501",
      };
    return {
      columns: [
        { name: "id", typeName: "INT8" },
        { name: "customer_email", typeName: "TEXT" },
        { name: "total", typeName: "NUMERIC" },
        { name: "status", typeName: "TEXT" },
      ],
      rows: [
        [v("90412"), v("amara.banda@example.com"), v("1240.00"), v("pending")],
        [v("90398"), v(null), v("86.50"), v("paid")],
        [v("9007199254740993"), v("zoë@example.com"), v("0.00"), v("refunded")],
      ],
      rowsAffected: null,
      truncated: false,
      extraResultSets: 0,
      elapsedMs: 12,
    };
  },
};

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
