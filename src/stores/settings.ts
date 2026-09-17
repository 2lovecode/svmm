import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "../api/tauri";
import {
  defaultSettings,
  formatAppError,
  type NexusUser,
  type Settings,
} from "../types/mod";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings>(defaultSettings());
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const message = ref<string | null>(null);

  const nexusKeyInput = ref("");
  const nexusHasKey = ref(false);
  const nexusUser = ref<NexusUser | null>(null);
  const nexusBusy = ref(false);

  async function load() {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await api.getSettings();
      await refreshNexusStatus();
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function save() {
    saving.value = true;
    error.value = null;
    message.value = null;
    try {
      await api.saveSettings({ ...settings.value });
      message.value = "设置已保存";
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      saving.value = false;
    }
  }

  async function discover() {
    error.value = null;
    message.value = null;
    try {
      const paths = await api.discoverPaths();
      if (!paths) {
        message.value = "未自动找到游戏目录，请手动选择";
        return;
      }
      settings.value = {
        ...settings.value,
        gamePath: paths.gamePath,
        smapiPath: paths.smapiPath,
        modsPath: paths.modsPath,
      };
      message.value = "已填入自动探测到的路径";
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    }
  }

  function patch(partial: Partial<Settings>) {
    settings.value = { ...settings.value, ...partial };
  }

  async function refreshNexusStatus() {
    const status = await api.nexusStatus();
    nexusHasKey.value = status.hasKey;
    if (!status.hasKey) {
      nexusUser.value = null;
    }
  }

  async function saveNexusKey() {
    nexusBusy.value = true;
    error.value = null;
    message.value = null;
    try {
      await api.nexusSetKey(nexusKeyInput.value);
      nexusKeyInput.value = "";
      await refreshNexusStatus();
      message.value = "Nexus API 密钥已保存";
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      nexusBusy.value = false;
    }
  }

  async function validateNexusKey() {
    nexusBusy.value = true;
    error.value = null;
    message.value = null;
    try {
      const user = await api.nexusValidate();
      nexusUser.value = user;
      nexusHasKey.value = true;
      const flags = [
        user.isPremium ? "Premium" : null,
        user.isSupporter ? "Supporter" : null,
      ]
        .filter(Boolean)
        .join(" · ");
      message.value = flags
        ? `已连接：${user.name}（${flags}）`
        : `已连接：${user.name}`;
    } catch (e) {
      nexusUser.value = null;
      error.value = formatAppError(e);
      throw e;
    } finally {
      nexusBusy.value = false;
    }
  }

  async function clearNexusKey() {
    nexusBusy.value = true;
    error.value = null;
    message.value = null;
    try {
      await api.nexusClearKey();
      nexusKeyInput.value = "";
      nexusUser.value = null;
      nexusHasKey.value = false;
      message.value = "Nexus API 密钥已清除";
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      nexusBusy.value = false;
    }
  }

  return {
    settings,
    loading,
    saving,
    error,
    message,
    nexusKeyInput,
    nexusHasKey,
    nexusUser,
    nexusBusy,
    load,
    save,
    discover,
    patch,
    saveNexusKey,
    validateNexusKey,
    clearNexusKey,
  };
});
