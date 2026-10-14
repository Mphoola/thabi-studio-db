import { ref, watch } from "vue";

/** @typedef {"explorer" | "saved" | "history"} SidebarView */

export const SIDEBAR_MIN = 200;
export const SIDEBAR_MAX = 360;
const SIDEBAR_DEFAULT = 240;
const COMPACT_BELOW = 1000;
const STORAGE_KEY = "thabi.layout";

function load() {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "{}");
  } catch {
    return {};
  }
}

const saved = load();
const clamp = (width) => Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, width));

const sidebarWidth = ref(clamp(Number(saved.sidebarWidth) || SIDEBAR_DEFAULT));
const sidebarPinned = ref(saved.sidebarOpen ?? true);
const sidebarView = ref(/** @type {SidebarView} */ ("explorer"));
const compact = ref(window.innerWidth < COMPACT_BELOW);
/** Which pane fills the main area. "gallery" exists only in dev builds. */
const mainView = ref(/** @type {"welcome" | "gallery"} */ ("welcome"));
// In a narrow window the sidebar starts hidden but can still be opened.
const sidebarOpen = ref(sidebarPinned.value && !compact.value);

window.addEventListener("resize", () => {
  const next = window.innerWidth < COMPACT_BELOW;
  if (next === compact.value) return;
  compact.value = next;
  sidebarOpen.value = next ? false : sidebarPinned.value;
});

watch([sidebarWidth, sidebarPinned], () => {
  try {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({
        sidebarWidth: sidebarWidth.value,
        sidebarOpen: sidebarPinned.value,
      }),
    );
  } catch {
    // Layout just won't be remembered.
  }
});

export function useLayout() {
  function toggleSidebar() {
    sidebarOpen.value = !sidebarOpen.value;
    if (!compact.value) sidebarPinned.value = sidebarOpen.value;
  }
  function setSidebarWidth(width) {
    sidebarWidth.value = clamp(Math.round(width));
  }
  return {
    sidebarOpen,
    sidebarWidth,
    sidebarView,
    compact,
    mainView,
    toggleSidebar,
    setSidebarWidth,
  };
}
