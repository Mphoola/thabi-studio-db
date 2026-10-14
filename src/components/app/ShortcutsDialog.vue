<script setup>
import { computed } from "vue";
import { useCommands } from "../../composables/useCommands.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import KbdShortcut from "../common/KbdShortcut.vue";

const open = defineModel("open", { type: Boolean, default: false });
const { commands } = useCommands();

const groups = computed(() => {
  const byGroup = new Map();
  for (const command of commands.value) {
    if (!command.shortcut) continue;
    const group = command.group ?? "General";
    byGroup.set(group, [...(byGroup.get(group) ?? []), command]);
  }
  return [...byGroup.entries()];
});
</script>

<template>
  <BaseDialog
    v-model:open="open"
    title="Keyboard Shortcuts"
    description="More shortcuts arrive as features land: running queries, tabs and connections."
  >
    <div class="flex flex-col gap-4">
      <section
        v-for="[group, list] in groups"
        :key="group"
        class="flex flex-col gap-1"
      >
        <h3
          class="text-xs font-semibold tracking-wide text-fg-subtle uppercase"
        >
          {{ group }}
        </h3>
        <dl class="flex flex-col">
          <div
            v-for="command in list"
            :key="command.id"
            class="flex h-7 items-center justify-between border-b border-line last:border-b-0"
          >
            <dt>{{ command.title }}</dt>
            <dd><KbdShortcut :shortcut="command.shortcut" /></dd>
          </div>
        </dl>
      </section>
    </div>
    <template #footer>
      <BaseButton variant="primary" @click="open = false">Done</BaseButton>
    </template>
  </BaseDialog>
</template>
