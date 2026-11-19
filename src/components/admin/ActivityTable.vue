<script setup>
import { computed, ref } from "vue";
import { CircleStop, OctagonX } from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import { useToasts } from "../../composables/useToasts.js";
import { toUserDetail } from "../../lib/errors.js";
import { formatDuration } from "../../lib/formatSize.js";
import { killProcess } from "../../lib/tauri/commands.js";
import IconButton from "../common/IconButton.vue";
import TextInput from "../common/TextInput.vue";

/**
 * Who is connected and what they are running, longest first, with Cancel
 * (the query) and End (the session), each confirmed. This app's own session
 * is marked and can't be stopped from here.
 */
const props = defineProps({
  processes: { type: Array, required: true },
  profileId: { type: String, required: true },
  engine: { type: String, default: "postgres" },
  readOnly: { type: Boolean, default: false },
  connection: { type: String, default: "" },
});
const emit = defineEmits(["changed"]);
const toasts = useToasts();
const filter = ref("");
const idleShown = ref(false);

const IDLE = /^(idle|sleep|sleeping)$/i;
const shown = computed(() => {
  const words = filter.value.trim().toLowerCase();
  return props.processes.filter(
    (p) =>
      (idleShown.value || !IDLE.test(p.state) || p.own) &&
      (!words ||
        [p.id, p.user, p.database, p.client, p.state, p.query]
          .join(" ")
          .toLowerCase()
          .includes(words)),
  );
});
const idleCount = computed(
  () => props.processes.filter((p) => IDLE.test(p.state) && !p.own).length,
);

async function stop(process, terminate) {
  const what = terminate ? "End session" : "Cancel query";
  const ok = await confirm({
    title: `${what} ${process.id}?`,
    message: terminate
      ? `${process.user}'s session on “${props.connection}” is closed. Anything it hasn't committed is rolled back.`
      : `${process.user}'s running statement on “${props.connection}” is stopped. The session stays open.`,
    confirmLabel: what,
    destructive: true,
  });
  if (!ok) return;
  try {
    await killProcess(props.profileId, process.id, terminate);
    toasts.info(terminate ? "Session ended" : "Query cancelled", process.id);
  } catch (error) {
    toasts.error(`Couldn’t ${what.toLowerCase()}`, toUserDetail(error));
  }
  emit("changed");
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col">
    <div class="flex items-center gap-3 px-4 py-2">
      <TextInput
        v-model="filter"
        class="w-64"
        placeholder="Filter by user, database or query"
        aria-label="Filter sessions"
      />
      <label class="flex items-center gap-1.5 text-sm text-fg-muted">
        <input v-model="idleShown" type="checkbox" />
        Show {{ idleCount }} idle
      </label>
      <span class="ml-auto text-sm text-fg-subtle"
        >{{ shown.length }} of {{ processes.length }} sessions</span
      >
    </div>
    <div class="min-h-0 flex-1 overflow-auto">
      <table class="w-full text-sm">
        <thead
          class="sticky top-0 bg-grid-header text-left text-xs text-fg-muted"
        >
          <tr>
            <th class="px-3 py-1.5 font-semibold">Id</th>
            <th class="px-3 py-1.5 font-semibold">User</th>
            <th class="px-3 py-1.5 font-semibold">Database</th>
            <th class="px-3 py-1.5 font-semibold">Client</th>
            <th class="px-3 py-1.5 font-semibold">State</th>
            <th class="px-3 py-1.5 font-semibold">For</th>
            <th class="px-3 py-1.5 font-semibold">Query</th>
            <th class="w-16 px-3 py-1.5" />
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="process in shown"
            :key="process.id"
            class="border-t border-line align-top"
          >
            <td class="px-3 py-1.5 font-mono text-xs">
              {{ process.id }}
              <span v-if="process.own" class="text-accent-text"
                >(this app)</span
              >
            </td>
            <td class="px-3 py-1.5">{{ process.user }}</td>
            <td class="px-3 py-1.5 text-fg-muted">{{ process.database }}</td>
            <td class="px-3 py-1.5 text-fg-muted">{{ process.client }}</td>
            <td class="px-3 py-1.5">
              {{ process.state }}
              <span
                v-if="process.waiting"
                class="block text-xs text-fg-subtle"
                >{{ process.waiting }}</span
              >
            </td>
            <td class="px-3 py-1.5 tabular-nums">
              {{ formatDuration(process.seconds) }}
            </td>
            <td class="max-w-md px-3 py-1.5">
              <code
                class="line-clamp-3 font-mono text-xs break-all text-fg-muted"
                >{{ process.query }}</code
              >
            </td>
            <td class="px-3 py-1.5">
              <div v-if="!process.own && !readOnly" class="flex gap-0.5">
                <IconButton
                  v-if="engine !== 'sqlserver'"
                  :icon="CircleStop"
                  size="sm"
                  :label="`Cancel query ${process.id}`"
                  @click="stop(process, false)"
                />
                <IconButton
                  :icon="OctagonX"
                  size="sm"
                  :label="`End session ${process.id}`"
                  @click="stop(process, true)"
                />
              </div>
            </td>
          </tr>
          <tr v-if="!shown.length">
            <td
              colspan="8"
              class="px-3 py-4 text-center text-sm text-fg-subtle"
            >
              No sessions match.
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
