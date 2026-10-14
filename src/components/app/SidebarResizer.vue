<script setup>
import {
  SIDEBAR_MAX,
  SIDEBAR_MIN,
  useLayout,
} from "../../composables/useLayout.js";

const { sidebarWidth, setSidebarWidth } = useLayout();

function onPointerDown(event) {
  const startX = event.clientX;
  const startWidth = sidebarWidth.value;
  const handle = event.currentTarget;
  handle.setPointerCapture(event.pointerId);
  const move = (e) => setSidebarWidth(startWidth + e.clientX - startX);
  const up = () => {
    handle.removeEventListener("pointermove", move);
    handle.removeEventListener("pointerup", up);
    document.body.style.cursor = "";
  };
  document.body.style.cursor = "col-resize";
  handle.addEventListener("pointermove", move);
  handle.addEventListener("pointerup", up);
}

function onKeydown(event) {
  const step = event.shiftKey ? 40 : 10;
  if (event.key === "ArrowLeft") setSidebarWidth(sidebarWidth.value - step);
  else if (event.key === "ArrowRight")
    setSidebarWidth(sidebarWidth.value + step);
  else return;
  event.preventDefault();
}
</script>

<template>
  <div
    role="separator"
    aria-orientation="vertical"
    aria-label="Resize sidebar"
    :aria-valuenow="sidebarWidth"
    :aria-valuemin="SIDEBAR_MIN"
    :aria-valuemax="SIDEBAR_MAX"
    tabindex="0"
    class="absolute inset-y-0 -right-1 z-10 w-2 cursor-col-resize after:absolute after:inset-y-0 after:left-1/2 after:w-px after:transition-colors hover:after:bg-accent focus-visible:after:bg-accent"
    @pointerdown="onPointerDown"
    @dblclick="setSidebarWidth(240)"
    @keydown="onKeydown"
  />
</template>
