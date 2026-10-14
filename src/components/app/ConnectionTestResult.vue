<script setup>
import { computed } from "vue";
import InlineAlert from "../common/InlineAlert.vue";

const props = defineProps({
  /** { ok: true, server, elapsedMs } or { ok: false, error } */
  result: { type: Object, required: true },
});

const title = computed(() =>
  props.result.ok
    ? `Connected to ${props.result.server.version} in ${props.result.elapsedMs.toLocaleString()} ms.`
    : (props.result.error?.message ?? "Couldn't connect."),
);
const detail = computed(() =>
  props.result.ok
    ? ""
    : [
        props.result.error?.detail,
        props.result.error?.code && `Code ${props.result.error.code}`,
      ]
        .filter(Boolean)
        .join(" · "),
);
</script>

<template>
  <InlineAlert
    :tone="result.ok ? 'success' : 'danger'"
    :title="title"
    :message="detail"
    mono
  />
</template>
