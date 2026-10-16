<script setup>
import { computed, ref } from "vue";
import { Bookmark, BookmarkPlus, Trash2 } from "lucide-vue-next";
import { confirm } from "../../composables/useConfirm.js";
import BaseButton from "../common/BaseButton.vue";
import BaseDialog from "../common/BaseDialog.vue";
import DropdownMenu from "../common/DropdownMenu.vue";
import IconButton from "../common/IconButton.vue";
import TextInput from "../common/TextInput.vue";

/**
 * Saved filter sets for this table.
 *
 * A preset is the filters the bar was holding when it was saved, so choosing
 * one is putting them back. Saving over a name already used here updates that
 * preset rather than making a second one that looks the same.
 */
const props = defineProps({
  presets: { type: Array, required: true },
  loading: { type: Boolean, default: false },
  /** Name of the preset the current filters match, if they match one. */
  currentName: { type: String, default: "" },
  /** Whether there is anything worth saving. */
  canSave: { type: Boolean, default: false },
});
const emit = defineEmits(["apply", "save", "remove"]);

const naming = ref(false);
const name = ref("");

const items = computed(() =>
  props.presets.map((preset) => ({ id: preset.id, label: preset.name })),
);

async function openNaming() {
  name.value = props.currentName;
  naming.value = true;
}

async function save() {
  if (!name.value.trim()) return;
  emit("save", name.value);
  naming.value = false;
}

function pick(id) {
  const preset = props.presets.find((one) => one.id === id);
  if (preset) emit("apply", preset);
}

/** Only offered for the preset the filters currently match. */
const removable = computed(() =>
  props.presets.find((preset) => preset.name === props.currentName),
);

async function remove() {
  if (!removable.value) return;
  const preset = removable.value;
  if (
    await confirm({
      title: `Delete “${preset.name}”?`,
      message: "The filters stay on the table; only the saved name goes.",
      confirmLabel: "Delete",
      destructive: true,
    })
  )
    emit("remove", preset);
}
</script>

<template>
  <div class="flex items-center gap-1">
    <DropdownMenu
      v-if="presets.length"
      :items="items"
      label="Saved filters"
      align="end"
      @select="pick"
    >
      <BaseButton size="sm" :icon="Bookmark" :disabled="loading">
        {{ presets.length }} saved
      </BaseButton>
    </DropdownMenu>
    <IconButton
      :icon="BookmarkPlus"
      label="Save These Filters"
      :disabled="!canSave"
      @click="openNaming"
    />
    <IconButton
      :icon="Trash2"
      label="Delete Saved Filters"
      :disabled="!removable"
      @click="remove"
    />

    <BaseDialog v-model:open="naming" size="sm" title="Save Filters">
      <form class="flex flex-col gap-3" @submit.prevent="save">
        <p class="text-sm text-fg-muted">
          {{
            currentName
              ? `These filters are saved as “${currentName}”. Saving under a different name adds another preset.`
              : "Saved filters belong to this table and are here after you close it."
          }}
        </p>
        <TextInput
          v-model="name"
          label="Name"
          placeholder="Recent orders"
          aria-label="Preset name"
        />
      </form>
      <template #footer>
        <BaseButton size="sm" @click="naming = false">Cancel</BaseButton>
        <BaseButton
          size="sm"
          variant="primary"
          :disabled="!name.trim()"
          @click="save"
        >
          Save
        </BaseButton>
      </template>
    </BaseDialog>
  </div>
</template>
