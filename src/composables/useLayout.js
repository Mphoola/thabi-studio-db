import { ref } from "vue";
import { persistAppSetting } from "./useAppSettings.js";

/** @typedef {"explorer" | "saved" | "history"} SidebarView */

export const SIDEBAR_MIN = 200;
export const SIDEBAR_MAX = 360;
const SIDEBAR_DEFAULT = 240;
const COMPACT_BELOW = 1000;
const clamp = (width) => Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, width));

const sidebarWidth = ref(SIDEBAR_DEFAULT);
const sidebarPinned = ref(true);
const sidebarView = ref(/** @type {SidebarView} */ ("explorer"));
const compact = ref(window.innerWidth < COMPACT_BELOW);
/** Which pane fills the main area. "gallery" exists only in dev builds. */
const mainView = ref(
  /** @type {"welcome" | "object" | "gallery"} */ ("welcome"),
);
// In a narrow window the sidebar starts hidden but can still be opened.
const sidebarOpen = ref(!compact.value);
let widthSaveTimer;

export function initLayout(settings) {
  sidebarWidth.value = clamp(Number(settings?.sidebarWidth) || SIDEBAR_DEFAULT);
  sidebarPinned.value = settings?.sidebarOpen ?? true;
  sidebarOpen.value = sidebarPinned.value && !compact.value;
}

window.addEventListener("resize", () => {
  const next = window.innerWidth < COMPACT_BELOW;
  if (next === compact.value) return;
  compact.value = next;
  sidebarOpen.value = next ? false : sidebarPinned.value;
});

export function useLayout() {
  function toggleSidebar() {
    sidebarOpen.value = !sidebarOpen.value;
    if (!compact.value) {
      sidebarPinned.value = sidebarOpen.value;
      persistAppSetting("sidebarOpen", sidebarPinned.value);
    }
  }
  function setSidebarWidth(width) {
    sidebarWidth.value = clamp(Math.round(width));
    clearTimeout(widthSaveTimer);
    widthSaveTimer = setTimeout(
      () => persistAppSetting("sidebarWidth", sidebarWidth.value),
      250,
    );
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
