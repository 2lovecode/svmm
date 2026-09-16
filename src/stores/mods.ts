import { defineStore } from "pinia";
import { computed, ref } from "vue";
import * as api from "../api/tauri";
import { formatAppError, type ModEntry } from "../types/mod";

export const useModsStore = defineStore("mods", () => {
  const mods = ref<ModEntry[]>([]);
  const loading = ref(false);
  const launching = ref(false);
  const togglingPath = ref<string | null>(null);
  const error = ref<string | null>(null);
  const statusMessage = ref("就绪");

  const enabledCount = computed(() => mods.value.filter((m) => m.enabled).length);
  const totalCount = computed(() => mods.value.length);

  async function refresh() {
    loading.value = true;
    error.value = null;
    statusMessage.value = "正在扫描模组…";
    try {
      mods.value = await api.scanMods();
      statusMessage.value = `已加载 ${mods.value.length} 个模组`;
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "扫描失败";
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function toggle(mod: ModEntry, enabled: boolean) {
    togglingPath.value = mod.folderPath;
    error.value = null;
    statusMessage.value = enabled ? `正在启用 ${mod.name}…` : `正在禁用 ${mod.name}…`;
    try {
      const updated = await api.setModEnabled(mod.folderPath, enabled);
      const idx = mods.value.findIndex((m) => m.folderPath === mod.folderPath);
      if (idx >= 0) {
        mods.value[idx] = updated;
      } else {
        await refresh();
      }
      statusMessage.value = enabled ? `已启用 ${updated.name}` : `已禁用 ${updated.name}`;
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "切换失败";
      throw e;
    } finally {
      togglingPath.value = null;
    }
  }

  async function launch() {
    launching.value = true;
    error.value = null;
    statusMessage.value = "正在启动 SMAPI…";
    try {
      await api.launchSmapi();
      statusMessage.value = "已启动 SMAPI";
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "启动失败";
      throw e;
    } finally {
      launching.value = false;
    }
  }

  return {
    mods,
    loading,
    launching,
    togglingPath,
    error,
    statusMessage,
    enabledCount,
    totalCount,
    refresh,
    toggle,
    launch,
  };
});
