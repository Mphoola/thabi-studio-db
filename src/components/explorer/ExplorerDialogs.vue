<script setup>
import ContainerDialog from "./ContainerDialog.vue";
import ImportDialog from "../import/ImportDialog.vue";
import RestoreSqlDialog from "../import/RestoreSqlDialog.vue";
import NativeBackupDialog from "../import/NativeBackupDialog.vue";
import { useContainerDialog } from "./useContainerDialog.js";
import { useImportDialog } from "./useImportDialog.js";

/**
 * The dialogs the explorer's menus open: import, run a SQL dump, and new or
 * dropped databases and schemas. Each is keyed by its target, so opening it
 * for another table starts it afresh.
 */
const emit = defineEmits(["changed"]);
const containers = useContainerDialog();
const imports = useImportDialog();
</script>

<template>
  <ImportDialog
    v-if="imports.target.value"
    :key="JSON.stringify(imports.target.value)"
    v-model:open="imports.open.value"
    :target="imports.target.value"
    @done="emit('changed')"
  />
  <RestoreSqlDialog
    v-if="imports.restoreTarget.value"
    :key="JSON.stringify(imports.restoreTarget.value)"
    v-model:open="imports.restoreOpen.value"
    :target="imports.restoreTarget.value"
    @done="emit('changed')"
  />
  <NativeBackupDialog
    v-if="imports.backupTarget.value"
    :key="JSON.stringify(imports.backupTarget.value)"
    v-model:open="imports.backupOpen.value"
    :target="imports.backupTarget.value"
  />
  <ContainerDialog
    v-if="containers.target.value"
    v-model:open="containers.open.value"
    :target="containers.target.value"
    @done="emit('changed')"
  />
</template>
