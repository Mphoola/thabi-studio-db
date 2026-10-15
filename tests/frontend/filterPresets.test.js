import { describe, expect, it, vi } from "vitest";
import { computed, shallowRef } from "vue";

const saved = [];
const calls = [];
vi.mock("../../src/lib/tauri/commands.js", () => ({
  listFilterPresets: vi.fn(async (...args) => {
    calls.push(["list", ...args]);
    return saved
      .filter((preset) => preset.objectName === args[3])
      .map((p) => ({
        ...p,
      }));
  }),
  saveFilterPreset: vi.fn(async (input) => {
    calls.push(["save", input]);
    const preset = { id: `p${saved.length + 1}`, ...input };
    saved.push(preset);
    return preset;
  }),
  deleteFilterPreset: vi.fn(async (id) => {
    calls.push(["delete", id]);
    const index = saved.findIndex((preset) => preset.id === id);
    if (index >= 0) saved.splice(index, 1);
  }),
}));

const { useFilterPresets } =
  await import("../../src/composables/useFilterPresets.js");

/**
 * A preset is scoped to one connection and one table. Getting that wrong shows
 * a production filter on staging, or "Recent orders" from another table, which
 * is worse than showing none.
 */
function setup(objectName = "orders") {
  saved.length = 0;
  calls.length = 0;
  const filters = shallowRef([
    { column: "status", op: "eq", value: "open", enabled: true },
  ]);
  const state = useFilterPresets({
    profileId: shallowRef("prod"),
    target: computed(() => ({
      database: "shop",
      schema: "public",
      name: objectName,
    })),
    filters,
    toasts: { success: () => {}, error: () => {} },
  });
  return { filters, state };
}

describe("filter presets belong to one table on one connection", () => {
  it("reads the presets for the table it is on", async () => {
    const { state } = setup();
    await state.refresh();
    expect(calls[0]).toEqual(["list", "prod", "shop", "public", "orders"]);
  });

  it("saves with the table's own scope", async () => {
    const { state } = setup();
    await state.save("Recent orders");
    expect(calls.find(([kind]) => kind === "save")[1]).toMatchObject({
      profileId: "prod",
      databaseName: "shop",
      schemaName: "public",
      objectName: "orders",
      name: "Recent orders",
    });
    expect(state.presets.value).toHaveLength(1);
  });

  it("keeps a table's presets off another table", async () => {
    const { state } = setup();
    await state.save("Recent orders");
    const other = setup("customers");
    await other.state.refresh();
    expect(other.state.presets.value).toHaveLength(0);
  });

  it("applies a preset back onto the filter bar", async () => {
    const { filters, state } = setup();
    await state.save("Recent orders");
    filters.value = [];
    state.apply(state.presets.value[0]);
    expect(filters.value).toEqual([
      { column: "status", op: "eq", value: "open", enabled: true },
    ]);
  });

  it("only saves the filters that are switched on", async () => {
    const { filters, state } = setup();
    filters.value = [
      { column: "status", op: "eq", value: "open", enabled: true },
      { column: "total", op: "gt", value: "", enabled: false },
    ];
    await state.save("Recent");
    const savedFilters = calls.find(([kind]) => kind === "save")[1].filters;
    expect(savedFilters).toHaveLength(1);
    expect(savedFilters[0].column).toBe("status");
  });

  it("recognises the filters already saved under a name", async () => {
    const { state } = setup();
    await state.save("Recent orders");
    expect(state.existingName()).toBe("Recent orders");
  });

  it("does not claim a name for filters that are not saved", async () => {
    const { filters, state } = setup();
    await state.save("Recent orders");
    filters.value = [
      { column: "status", op: "eq", value: "closed", enabled: true },
    ];
    expect(state.existingName()).toBe("");
  });

  it("refuses to save nothing", async () => {
    const { filters, state } = setup();
    filters.value = [];
    expect(await state.save("Empty")).toBeNull();
    expect(state.presets.value).toHaveLength(0);
  });

  it("deletes a preset and forgets it", async () => {
    const { state } = setup();
    const preset = (await state.save("Recent orders"), state.presets.value[0]);
    await state.remove(preset);
    expect(state.presets.value).toHaveLength(0);
  });
});
