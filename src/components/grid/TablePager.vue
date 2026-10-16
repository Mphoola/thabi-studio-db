<script setup>
import { computed } from "vue";
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from "lucide-vue-next";
import { describeRows, pageCount } from "../../lib/tablePaging.js";
import IconButton from "../common/IconButton.vue";

/**
 * Moving through a table a page at a time.
 *
 * The page size is the connection's row limit, so a connection set to 10,000
 * rows moves 10,000 at a time. Scrolling still runs forward on its own; these
 * are for going back, or jumping to the end of a table with a million rows.
 */
const props = defineProps({
  /** Zero-based page on screen. */
  page: { type: Number, required: true },
  pageSize: { type: Number, required: true },
  /** Rows the table has; absent when the estimate failed and no count was asked for. */
  total: { type: Number, default: null },
  /** Whether `total` is the catalog's guess rather than a count. */
  estimated: { type: Boolean, default: false },
  /**
   * Whether rows are known to remain past the window. An estimate can say the
   * last page when a short page would prove otherwise, so "next" follows what
   * the server actually returned.
   */
  hasMore: { type: Boolean, default: true },
  disabled: { type: Boolean, default: false },
});
const emit = defineEmits(["go"]);

/** Pages in total, or null while the size is unknown. */
const pages = computed(() => pageCount(props.total, props.pageSize));
const last = computed(() => (pages.value ? pages.value - 1 : null));
const firstPage = computed(() => props.page === 0);
const lastPage = computed(() =>
  props.estimated || !pages.value ? false : props.page >= last.value,
);
</script>

<template>
  <div class="flex items-center gap-1">
    <span class="mr-2 text-xs text-fg-muted tabular-nums">
      {{ describeRows(page, pageSize, total ?? Infinity, estimated) }}
      <template v-if="pages">
        · Page {{ page + 1 }} of {{ pages.toLocaleString() }}
      </template>
    </span>
    <IconButton
      size="sm"
      :icon="ChevronsLeft"
      label="First page"
      :disabled="disabled || firstPage"
      @click="emit('go', 0)"
    />
    <IconButton
      size="sm"
      :icon="ChevronLeft"
      label="Previous page"
      :disabled="disabled || firstPage"
      @click="emit('go', page - 1)"
    />
    <IconButton
      size="sm"
      :icon="ChevronRight"
      label="Next page"
      :disabled="disabled || (lastPage && !hasMore)"
      @click="emit('go', page + 1)"
    />
    <IconButton
      size="sm"
      :icon="ChevronsRight"
      label="Last page"
      :disabled="disabled || !pages || lastPage"
      @click="emit('go', last)"
    />
  </div>
</template>
