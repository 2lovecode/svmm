import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import * as api from "../api/tauri";
import { formatAppError, type ModEntry, type SmapiInstallProgress, type UpdateInfo } from "../types/mod";

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
  const installingSmapi = ref(false);
  const smapiProgress = ref<SmapiInstallProgress | null>(null);
  const smapiInstalled = ref<boolean | null>(null);
  const gameFound = ref(false);
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

  async function refreshSmapiStatus() {
    try {
      const status = await api.smapiStatus();
      smapiInstalled.value = status.installed;
      gameFound.value = status.gameFound;
    } catch (e) {
      error.value = formatAppError(e);
    }
  }

  async function installSmapi() {
    installingSmapi.value = true;
    error.value = null;
    smapiProgress.value = {
      phase: "query",
      message: "正在获取 SMAPI 版本…",
      received: 0,
      total: null,
      percent: null,
    };
    statusMessage.value = smapiProgress.value.message;
    let unlisten: UnlistenFn | null = null;
    try {
      unlisten = await listen<SmapiInstallProgress>("smapi-install-progress", (event) => {
        smapiProgress.value = event.payload;
        statusMessage.value = event.payload.message;
      });
    } catch {
      unlisten = null;
    }
    try {
      const report = await api.installSmapi();
      smapiInstalled.value = true;
      gameFound.value = true;
      try {
        await refresh();
        statusMessage.value = `已安装 SMAPI ${report.version}`;
      } catch {
        statusMessage.value = `已安装 SMAPI ${report.version}，但模组列表刷新失败`;
      }
      return report;
    } catch (e) {
      error.value = formatAppError(e);
      statusMessage.value = "SMAPI 安装失败";
      throw e;
    } finally {
      if (unlisten) unlisten();
      smapiProgress.value = null;
      installingSmapi.value = false;
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

  async function installZip(path: string) {
    loading.value = true;
    error.value = null;
    statusMessage.value = "正在安装压缩包…";
    try {
      const entry = await api.installModZip(path);
      await refresh();
      statusMessage.value = `已安装：${entry.name}`;
      return entry;
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "安装失败";
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function installNxm(url: string) {
    loading.value = true;
    error.value = null;
    statusMessage.value = "正在通过 NXM 下载安装…";
    try {
      const entry = await api.installFromNxm(url);
      await refresh();
      statusMessage.value = `已通过 NXM 安装：${entry.name}`;
      return entry;
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "NXM 安装失败";
      throw e;
    } finally {
      loading.value = false;
    }
  }

  function nexusModId(mod: ModEntry): number | null {
    for (const key of mod.updateKeys) {
      const m = key.trim().match(/^nexus:\s*(\d+)$/i);
      if (m) return Number(m[1]);
    }
    return null;
  }

  const actionPath = ref<string | null>(null);

  async function endorse(mod: ModEntry) {
    const id = nexusModId(mod);
    if (id == null) {
      error.value = "该模组没有 Nexus 更新键";
      return;
    }
    actionPath.value = mod.folderPath;
    error.value = null;
    statusMessage.value = `正在推荐 ${mod.name}…`;
    try {
      await api.nexusEndorse(id, mod.version || null);
      statusMessage.value = `已推荐：${mod.name}`;
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "推荐失败";
      throw e;
    } finally {
      actionPath.value = null;
    }
  }

  async function updateFromNexus(mod: ModEntry) {
    if (nexusModId(mod) == null) {
      error.value = "该模组没有 Nexus 更新键";
      return;
    }
    actionPath.value = mod.folderPath;
    error.value = null;
    statusMessage.value = `正在从 Nexus 更新 ${mod.name}…`;
    try {
      const entry = await api.nexusUpdateMod(mod.folderPath);
      await refresh();
      statusMessage.value = `已更新：${entry.name}`;
      return entry;
    } catch (e) {
      const msg = formatAppError(e);
      error.value = msg;
      statusMessage.value = "更新失败";
      throw e;
    } finally {
      actionPath.value = null;
    }
  }

  return {
    mods,
    loading,
    checkingUpdates,
    launching,
    installingSmapi,
    smapiProgress,
    smapiInstalled,
    gameFound,
    togglingPath,
    actionPath,
    error,
    statusMessage,
    enabledCount,
    totalCount,
    refresh,
    refreshSmapiStatus,
    checkUpdates,
    toggle,
    launch,
    installSmapi,
    installZip,
    installNxm,
    endorse,
    updateFromNexus,
  };
});
