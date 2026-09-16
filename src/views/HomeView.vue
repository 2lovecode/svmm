<script setup lang="ts">
import { onMounted } from "vue";
import TopBar from "../components/TopBar.vue";
import ModTable from "../components/ModTable.vue";
import StatusBar from "../components/StatusBar.vue";
import { useModsStore } from "../stores/mods";
import type { ModEntry } from "../types/mod";

const mods = useModsStore();

onMounted(async () => {
  try {
    await mods.refresh();
  } catch {
    /* shown in status bar */
  }
});

async function onToggle(mod: ModEntry, enabled: boolean) {
  try {
    await mods.toggle(mod, enabled);
  } catch {
    /* shown in status bar */
  }
}
</script>

<template>
  <div class="page home-page">
    <TopBar :busy="mods.loading || mods.launching || !!mods.togglingPath" />
    <main class="page-body">
      <ModTable
        :mods="mods.mods"
        :loading="mods.loading"
        :toggling-path="mods.togglingPath"
        @toggle="onToggle"
      />
    </main>
    <StatusBar
      :message="mods.statusMessage"
      :enabled-count="mods.enabledCount"
      :total-count="mods.totalCount"
      :error="mods.error"
    />
  </div>
</template>
