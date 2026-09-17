import { defineStore } from "pinia";
import { computed, ref } from "vue";
import * as api from "../api/tauri";
import { formatAppError, type ModEntry, type UpdateInfo } from "../types/mod";

function mergeUpdateStatus(current: string, updateStatus: string): string {
  if (
    (current === "missing_manifest" || current === "incompatible") &&
    updateStatus !== "broken"
  ) {
    return current;
  }
  return updateStatus;
}

function applyUpdates(mods: ModEntry[], updates: UpdateInfo[]): ModEntry[] {
  const byId = new Map(updates.map((u) => [u.id, u]));
  return mods.map((mod) => {
    const info = byId.get(mod.id);
    if (!info) return mod;
    return {
      ...mod,
      status: mergeUpdateStatus(mod.status, info.status),
    };
  });
}

export const useModsStore = defineStore("mods", () => {
  const mods = ref<ModEntry[]>([]);
  const loading = ref(false);
  const checkingUpdates = ref(false);
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

  async function checkUpdates() {
    checkingUpdates.value = true;
    error.value = null;
    statusMessage.value = "正在检查更新…";
    try {
      const updates = await api.checkModUpdates();
      mods.value = applyUpdates(mods.value, updates);
      const available = updates.filter(
        (u) => u.status === "update_available" || u.status === "unofficial_update",
      ).length;
      statusMessage.value =
        available > 0
          ? `检查完成：${available} 个模组有更新`
          : "检查完成：暂无可用更新";
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "检查更新失败";
      throw e;
    } finally {
      checkingUpdates.value = false;
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
    checkingUpdates,
    launching,
    togglingPath,
    error,
    statusMessage,
    enabledCount,
    totalCount,
    refresh,
    checkUpdates,
    toggle,
    launch,
  };
});
