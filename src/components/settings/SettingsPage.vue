<script setup>
import { computed, nextTick, onMounted, ref } from "vue";
import {
  History,
  Palette,
  ShieldCheck,
  SquareCode,
  Table2,
} from "lucide-vue-next";
import { useLayout } from "../../composables/useLayout.js";
import BaseButton from "../common/BaseButton.vue";
import EditorSettings from "./EditorSettings.vue";
import GeneralSettings from "./GeneralSettings.vue";
import HistorySettings from "./HistorySettings.vue";
import ResultsSettings from "./ResultsSettings.vue";
import SecuritySettings from "./SecuritySettings.vue";

/**
 * Settings as a page of its own: sections on the left, one shown at a time,
 * like the system's settings. Done or Escape goes back to where you were.
 */
const SECTIONS = [
  {
    id: "general",
    label: "General",
    icon: Palette,
    component: GeneralSettings,
    description: "How Thabi Studio looks.",
  },
  {
    id: "security",
    label: "Security",
    icon: ShieldCheck,
    component: SecuritySettings,
    description: "The app password, idle lock and ways to recover it.",
  },
  {
    id: "editor",
    label: "Editor",
    icon: SquareCode,
    component: EditorSettings,
    description: "The SQL editor’s font and typing behaviour.",
  },
  {
    id: "results",
    label: "Queries & Results",
    icon: Table2,
    component: ResultsSettings,
    description: "Timeouts, how many rows are kept and how values are shown.",
  },
  {
    id: "history",
    label: "History",
    icon: History,
    component: HistorySettings,
    description: "What is remembered about the SQL you run.",
  },
];

const { settingsSection, closeSettings } = useLayout();
const section = computed(
  () => SECTIONS.find((s) => s.id === settingsSection.value) ?? SECTIONS[0],
);
const page = ref(null);

function select(id) {
  settingsSection.value = id;
  nextTick(() => page.value?.querySelector("[data-content]")?.scrollTo(0, 0));
}

// Arrow keys move between sections, as in a list.
function onNavKeydown(event) {
  if (!["ArrowDown", "ArrowUp"].includes(event.key)) return;
  event.preventDefault();
  const index = SECTIONS.indexOf(section.value);
  const next =
    SECTIONS[
      (index + (event.key === "ArrowDown" ? 1 : -1) + SECTIONS.length) %
        SECTIONS.length
    ];
  select(next.id);
  nextTick(() => page.value?.querySelector(`#settings-${next.id}`)?.focus());
}

function onKeydown(event) {
  // Escape inside one of the page's own dialogs closes only that dialog.
  if (event.key !== "Escape" || event.target.closest?.("dialog")) return;
  closeSettings();
}

onMounted(() =>
  page.value?.querySelector(`#settings-${section.value.id}`)?.focus(),
);
</script>

<template>
  <div ref="page" class="flex h-full min-h-0" @keydown="onKeydown">
    <nav
      class="flex w-52 shrink-0 flex-col gap-0.5 border-r border-line bg-sidebar p-3"
      aria-label="Settings sections"
    >
      <h1 class="px-2 pt-1 pb-3 text-lg font-semibold">Settings</h1>
      <button
        v-for="item in SECTIONS"
        :id="`settings-${item.id}`"
        :key="item.id"
        type="button"
        class="flex h-7 items-center gap-2 rounded-md px-2 text-left text-base transition-colors focus-visible:ring-3 focus-visible:ring-focus focus-visible:outline-none"
        :class="
          item.id === section.id
            ? 'bg-surface-active text-fg'
            : 'text-fg-muted hover:bg-surface-hover hover:text-fg'
        "
        :aria-current="item.id === section.id ? 'page' : undefined"
        @click="select(item.id)"
        @keydown="onNavKeydown"
      >
        <component
          :is="item.icon"
          :size="14"
          :stroke-width="1.75"
          aria-hidden="true"
        />
        {{ item.label }}
      </button>
    </nav>

    <section
      data-content
      class="min-w-0 flex-1 overflow-y-auto"
      :aria-labelledby="`settings-heading-${section.id}`"
    >
      <div class="mx-auto flex max-w-2xl flex-col gap-6 px-8 py-6">
        <header class="flex items-start justify-between gap-4">
          <div class="flex flex-col gap-0.5">
            <h2
              :id="`settings-heading-${section.id}`"
              class="text-xl font-semibold"
            >
              {{ section.label }}
            </h2>
            <p class="text-sm text-fg-subtle">{{ section.description }}</p>
          </div>
          <BaseButton variant="primary" @click="closeSettings">Done</BaseButton>
        </header>
        <component :is="section.component" />
      </div>
    </section>
  </div>
</template>
