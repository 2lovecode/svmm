<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

const dragOver = ref(false);
let unlistenNxm: UnlistenFn | null = null;

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

  try {
    unlistenNxm = await listen<{
      ok: boolean;
      message?: string;
      detail?: string | null;
      mod?: ModEntry;
    }>("nxm-install-result", async (event) => {
      const payload = event.payload;
      if (payload.ok) {
        mods.error = null;
        mods.statusMessage = payload.message ?? "NXM 安装成功";
        try {
          await mods.refresh();
        } catch {
          /* keep message */
        }
      } else {
        const detail = payload.detail ? `（${payload.detail}）` : "";
        mods.error = `${payload.message ?? "NXM 安装失败"}${detail}`;
        mods.statusMessage = "NXM 安装失败";
      }
    });
  } catch {
    /* event listen optional outside Tauri */
  }
});

onUnmounted(() => {
  if (unlistenNxm) {
    unlistenNxm();
    unlistenNxm = null;
  }
});

async function onToggle(mod: ModEntry, enabled: boolean) {
  try {
    await mods.toggle(mod, enabled);
  } catch {
    /* shown in status bar */
  }
}

async function onEndorse(mod: ModEntry) {
  try {
    await mods.endorse(mod);
  } catch {
    /* shown in status bar */
  }
}

async function onUpdate(mod: ModEntry) {
  try {
    await mods.updateFromNexus(mod);
  } catch {
    /* shown in status bar */
  }
}

function onDragOver(e: DragEvent) {
  e.preventDefault();
  dragOver.value = true;
}

function onDragLeave(e: DragEvent) {
  e.preventDefault();
  dragOver.value = false;
}

async function onDrop(e: DragEvent) {
  e.preventDefault();
  dragOver.value = false;
  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;

  const file = files[0] as File & { path?: string };
  const name = file.name?.toLowerCase() ?? "";
  if (!name.endsWith(".zip")) {
    mods.error = "仅支持拖放 .zip 模组压缩包";
    mods.statusMessage = "安装失败";
    return;
  }
  const path = file.path;
  if (!path) {
    mods.error = "无法获取文件路径，请确认在桌面应用中拖放";
    mods.statusMessage = "安装失败";
    return;
  }
  try {
    await mods.installZip(path);
  } catch {
    /* shown in status bar */
  }
}
</script>

<template>
  <div
    class="page home-page"
    :class="{ 'drag-over': dragOver }"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <TopBar
      :busy="
        mods.loading ||
        mods.checkingUpdates ||
        mods.launching ||
        !!mods.togglingPath ||
        !!mods.actionPath ||
        profiles.applying
      "
    />
    <main class="page-body">
      <p v-if="dragOver" class="drop-hint">松开以安装 .zip 模组</p>
      <ModTable
        :mods="mods.mods"
        :loading="mods.loading"
        :toggling-path="mods.togglingPath"
        :action-path="mods.actionPath"
        @toggle="onToggle"
        @endorse="onEndorse"
        @update="onUpdate"
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
