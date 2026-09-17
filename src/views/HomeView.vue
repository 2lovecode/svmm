<script setup lang="ts">
import { onMounted } from "vue";
import TopBar from "../components/TopBar.vue";
import ModTable from "../components/ModTable.vue";
import StatusBar from "../components/StatusBar.vue";
import { useModsStore } from "../stores/mods";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";
import type { ModEntry } from "../types/mod";

const mods = useModsStore();
const profiles = useProfilesStore();
const settings = useSettingsStore();

onMounted(async () => {
  try {
    await settings.load();
  } catch {
    /* optional */
  }
  try {
    await mods.refresh();
    if (settings.settings.checkUpdatesOnStartup) {
      try {
        await mods.checkUpdates();
      } catch {
        /* error in store */
      }
    }
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
    <TopBar
      :busy="
        mods.loading ||
        mods.checkingUpdates ||
        mods.launching ||
        !!mods.togglingPath ||
        profiles.applying
      "
    />
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
