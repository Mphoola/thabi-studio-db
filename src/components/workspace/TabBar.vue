<script setup>
import { Plus, X } from "lucide-vue-next";
import { runCommand } from "../../composables/useCommands.js";
import { useTabs } from "../../composables/useTabs.js";
import { tabIcon, tabTitle } from "../../lib/tabPresentation.js";
import IconButton from "../common/IconButton.vue";
import SpinnerIcon from "../common/SpinnerIcon.vue";

const { tabs, activeId, close, isDirty } = useTabs();

function onKeydown(event, index) {
  const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
  if (!step) return;
  event.preventDefault();
  const next =
    tabs.value[(index + step + tabs.value.length) % tabs.value.length];
  activeId.value = next.id;
  event.currentTarget.parentElement.children[tabs.value.indexOf(next)]?.focus();
}
</script>

<template>
  <div
    class="flex h-[34px] shrink-0 items-stretch border-b border-line bg-toolbar"
  >
    <div
      role="tablist"
      aria-label="Open tabs"
      class="flex min-w-0 overflow-x-auto"
    >
      <div
        v-for="(tab, index) in tabs"
        :key="tab.id"
        role="tab"
        :aria-selected="activeId === tab.id"
        :tabindex="activeId === tab.id ? 0 : -1"
        class="group flex max-w-56 shrink-0 cursor-default items-center gap-2 border-r border-line pr-1.5 pl-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-inset"
        :class="
          activeId === tab.id
            ? 'bg-canvas text-fg shadow-[inset_0_-2px_0_var(--accent)]'
            : 'text-fg-muted hover:text-fg'
        "
        @click="activeId = tab.id"
        @auxclick.middle="close(tab.id)"
        @keydown="onKeydown($event, index)"
      >
        <SpinnerIcon v-if="tab.running" :size="11" label="Running" />
        <component
          :is="tabIcon(tab)"
          v-else
          :size="13"
          :stroke-width="1.75"
          class="shrink-0 text-fg-subtle"
          aria-hidden="true"
        />
        <span class="truncate">{{ tabTitle(tab) }}</span>
        <span
          v-if="isDirty(tab)"
          class="size-1.5 shrink-0 rounded-full bg-fg-muted group-hover:hidden"
          aria-label="Not saved"
        />
        <button
          type="button"
          class="flex size-4 shrink-0 items-center justify-center rounded-sm text-fg-subtle opacity-0 group-hover:opacity-100 hover:bg-surface-active hover:text-fg focus-visible:opacity-100"
          :class="activeId === tab.id && !isDirty(tab) && 'opacity-100'"
          :aria-label="`Close ${tabTitle(tab)}`"
          @click.stop="close(tab.id)"
        >
          <X :size="11" :stroke-width="2" />
        </button>
      </div>
    </div>
    <IconButton
      :icon="Plus"
      size="sm"
      class="mx-1 self-center"
      label="New Query Tab"
      shortcut="Mod+T"
      @click="runCommand('tabs.newQuery')"
    />
  </div>
</template>
