<script setup>
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const greetMsg = ref("");
const name = ref("");

async function greet() {
  greetMsg.value = await invoke("greet", { name: name.value });
}
</script>

<template>
  <main
    class="flex min-h-screen flex-col items-center justify-center gap-6 bg-zinc-50 text-zinc-900 dark:bg-zinc-900 dark:text-zinc-100"
  >
    <h1 class="text-3xl font-semibold tracking-tight">Thabi Studio</h1>
    <p class="text-sm text-zinc-500">Tauri 2 · Vue · TypeScript · Tailwind</p>

    <form class="flex gap-2" @submit.prevent="greet">
      <input
        v-model="name"
        placeholder="Enter a name..."
        class="rounded-md border border-zinc-300 bg-white px-3 py-1.5 text-sm outline-none focus:ring-2 focus:ring-blue-500 dark:border-zinc-700 dark:bg-zinc-800"
      />
      <button
        type="submit"
        class="rounded-md bg-blue-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-blue-500"
      >
        Greet
      </button>
    </form>
    <p class="text-sm">{{ greetMsg }}</p>
  </main>
</template>
