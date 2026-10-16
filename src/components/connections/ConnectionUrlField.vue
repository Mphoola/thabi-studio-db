<script setup>
import { ref } from "vue";
import { Link } from "lucide-vue-next";
import { parseConnectionUrl } from "../../lib/connectionUrl.js";
import BaseButton from "../common/BaseButton.vue";
import TextInput from "../common/TextInput.vue";

/**
 * Paste a URL like postgres://user:pass@host:5432/db to fill the form.
 * Emits the parsed fields; the URL itself is never stored.
 */
const emit = defineEmits(["fill"]);
/** Without its own label and hint, for a row that already has a label. */
defineProps({ bare: { type: Boolean, default: false } });
const url = ref("");
const error = ref("");

function fill() {
  const fields = parseConnectionUrl(url.value);
  if (!fields) {
    error.value =
      "That doesn't look like a PostgreSQL, MySQL or SQL Server URL.";
    return;
  }
  error.value = "";
  emit("fill", fields);
  url.value = "";
}

function onPaste(event) {
  const text = event.clipboardData?.getData("text") ?? "";
  if (parseConnectionUrl(text)) {
    event.preventDefault();
    url.value = text;
    fill();
  }
}
</script>

<template>
  <div class="flex gap-2" :class="bare ? 'items-start' : 'items-end'">
    <TextInput
      v-model="url"
      class="flex-1"
      :label="bare ? '' : 'Connection URL'"
      :aria-label="bare ? 'Connection URL' : ''"
      placeholder="postgres://user:password@host:5432/database"
      :icon="Link"
      :error="error"
      :hint="bare ? '' : 'Optional. Paste a URL to fill in the fields below.'"
      mono
      @paste="onPaste"
      @keydown.enter.prevent="fill"
    />
    <BaseButton :class="!bare && 'mb-5'" :disabled="!url.trim()" @click="fill"
      >Fill In</BaseButton
    >
  </div>
</template>
