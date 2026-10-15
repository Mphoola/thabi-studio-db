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
  <div class="flex items-end gap-2">
    <TextInput
      v-model="url"
      class="flex-1"
      label="Connection URL"
      placeholder="postgres://user:password@host:5432/database"
      :icon="Link"
      :error="error"
      hint="Optional. Paste a URL to fill in the fields below."
      mono
      @paste="onPaste"
      @keydown.enter.prevent="fill"
    />
    <BaseButton class="mb-5" :disabled="!url.trim()" @click="fill"
      >Fill In</BaseButton
    >
  </div>
</template>
