import { computed, reactive, ref } from "vue";
import {
  applyTableDdl,
  cancelImport,
  describeObject,
  importProgress,
  importRows,
  previewImport,
} from "../lib/tauri/commands.js";
import { toUserDetail } from "../lib/errors.js";
import { autoMap, formatFor, toMappings } from "../lib/importMapping.js";

/**
 * The import dialog's state: the file and how to read it, its preview, which
 * file column fills which table column, and the run with its progress.
 *
 * `target` is { profileId, database, schema, table? }; with no table, the
 * import creates one from the file's columns first.
 */
export function useImport(target) {
  const tabId = `import:${target.database}:${target.schema ?? ""}`;
  const path = ref("");
  const options = reactive({
    format: "csv",
    delimiter: "",
    quote: '"',
    header: true,
    encoding: "utf8",
    emptyAsNull: true,
    skipBadRows: false,
  });
  const preview = ref(null);
  const previewError = ref("");
  const tableColumns = ref([]);
  /** For each table column, the file column index, or -1. */
  const chosen = ref([]);
  /** A new table: its name and each file column's name and type. */
  const newTable = reactive({ name: "", columns: [] });
  const running = ref(false);
  const status = ref(null);
  const summary = ref(null);
  const runError = ref("");

  const readOptions = () => ({
    format: options.format,
    delimiter: options.delimiter,
    quote: options.quote,
    header: options.header,
    encoding: options.encoding,
  });

  async function choose(file) {
    path.value = file;
    options.format = formatFor(file);
    options.delimiter = "";
    await reload();
  }

  async function reload() {
    if (!path.value) return;
    previewError.value = "";
    try {
      preview.value = await previewImport(
        target.profileId,
        path.value,
        readOptions(),
      );
      if (!options.delimiter && options.format === "csv")
        options.delimiter = preview.value.delimiter;
      newTable.columns = preview.value.columns.map((name, i) => ({
        name,
        type: preview.value.suggestedTypes[i],
        include: true,
      }));
      if (target.table) {
        if (!tableColumns.value.length) {
          const structure = await describeObject(target.profileId, {
            database: target.database,
            schema: target.schema,
            name: target.table,
            kind: "table",
          });
          tableColumns.value = structure.columns.map((c) => c.name);
        }
        chosen.value = autoMap(preview.value.columns, tableColumns.value);
      }
    } catch (error) {
      preview.value = null;
      previewError.value = toUserDetail(error);
    }
  }

  const mappings = computed(() =>
    target.table
      ? toMappings(tableColumns.value, chosen.value)
      : newTable.columns
          .map((c, fileColumn) => ({ column: c.name.trim(), fileColumn, c }))
          .filter((m) => m.c.include && m.column)
          .map(({ column, fileColumn }) => ({ column, fileColumn })),
  );

  /** Makes the new table: the file's chosen columns, all nullable. */
  async function createTable() {
    const columns = newTable.columns
      .filter((c) => c.include && c.name.trim())
      .map((c) => ({
        name: c.name.trim(),
        dataType: c.type.trim(),
        nullable: true,
        primaryKey: false,
      }));
    await applyTableDdl(
      target.profileId,
      tabId,
      {
        database: target.database,
        schema: target.schema || null,
        name: newTable.name.trim(),
      },
      { create: { columns } },
    );
  }

  async function run() {
    running.value = true;
    runError.value = "";
    summary.value = null;
    status.value = null;
    const poll = setInterval(async () => {
      status.value =
        (await importProgress(tabId).catch(() => null)) ?? status.value;
    }, 400);
    try {
      if (!target.table) await createTable();
      summary.value = await importRows(
        target.profileId,
        tabId,
        {
          database: target.database,
          schema: target.schema || null,
          table: target.table || newTable.name.trim(),
        },
        mappings.value,
        {
          ...readOptions(),
          emptyAsNull: options.emptyAsNull,
          skipBadRows: options.skipBadRows,
        },
        path.value,
      );
    } catch (error) {
      runError.value = toUserDetail(error);
    } finally {
      clearInterval(poll);
      running.value = false;
    }
  }

  const cancel = () => cancelImport(tabId);

  return {
    path,
    options,
    preview,
    previewError,
    tableColumns,
    chosen,
    newTable,
    mappings,
    running,
    status,
    summary,
    runError,
    choose,
    reload,
    run,
    cancel,
  };
}
