<script setup>
import { computed } from "vue";
import { Minus } from "lucide-vue-next";
import BaseButton from "../../common/BaseButton.vue";
import IconButton from "../../common/IconButton.vue";

/**
 * A file chosen with the system picker: a button that shows the chosen
 * file's name, and a remove button. Only the path is ever stored.
 */
const path = defineModel({ type: String, default: "" });
const props = defineProps({
  label: { type: String, required: true },
  /** async () => path | null */
  pick: { type: Function, required: true },
});
const name = computed(() => path.value.split(/[\\/]/).pop());

async function choose() {
  const chosen = await props.pick();
  if (chosen) path.value = chosen;
}
</script>

<template>
  <div class="flex min-w-0 items-center gap-1">
    <BaseButton
      size="sm"
      class="min-w-0 flex-1"
      :title="path || undefined"
      @click="choose"
    >
      <span class="truncate" :class="path && 'font-mono'">
        {{ path ? name : `${label}…` }}
      </span>
    </BaseButton>
    <IconButton
      v-if="path"
      :icon="Minus"
      size="sm"
      :label="`Remove ${label}`"
      @click="path = ''"
    />
  </div>
</template>
