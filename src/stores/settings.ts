import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "../api/tauri";
import {
  defaultSettings,
  formatAppError,
  type Settings,
} from "../types/mod";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings>(defaultSettings());
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const message = ref<string | null>(null);

  async function load() {
    loading.value = true;
    error.value = null;
    try {
      settings.value = await api.getSettings();
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

  return {
    settings,
    loading,
    saving,
    error,
    message,
    load,
    save,
    discover,
    patch,
  };
});
