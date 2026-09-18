import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "../api/tauri";
import {
  formatAppError,
  type ApplyReport,
  type Profile,
} from "../types/mod";

export const useProfilesStore = defineStore("profiles", () => {
  const profiles = ref<Profile[]>([]);
  const selectedId = ref<string | null>(null);
  const loading = ref(false);
  const applying = ref(false);
  const mutating = ref(false);
  const error = ref<string | null>(null);
  const message = ref<string | null>(null);
  const lastReport = ref<ApplyReport | null>(null);

  async function load(preferredId?: string | null) {
    loading.value = true;
    error.value = null;
    try {
      profiles.value = await api.listProfiles();
      const want =
        preferredId ??
        selectedId.value ??
        profiles.value.find((p) => p.id === "default")?.id ??
        profiles.value[0]?.id ??
        null;
      selectedId.value =
        want && profiles.value.some((p) => p.id === want)
          ? want
          : (profiles.value[0]?.id ?? null);
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function create(name: string) {
    mutating.value = true;
    error.value = null;
    message.value = null;
    try {
      const profile = await api.createProfile(name.trim());
      await load(profile.id);
      message.value = `已创建配置「${profile.name}」`;
      return profile;
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      mutating.value = false;
    }
  }

  async function snapshot(name: string) {
    mutating.value = true;
    error.value = null;
    message.value = null;
    try {
      const profile = await api.snapshotCurrentAsProfile(name.trim());
      await load(profile.id);
      message.value = `已快照当前启用模组为「${profile.name}」`;
      return profile;
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      mutating.value = false;
    }
  }

  async function rename(id: string, name: string) {
    mutating.value = true;
    error.value = null;
    message.value = null;
    try {
      const profile = await api.renameProfile(id, name.trim());
      await load(profile.id);
      message.value = `已重命名为「${profile.name}」`;
      return profile;
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      mutating.value = false;
    }
  }

  async function remove(id: string) {
    mutating.value = true;
    error.value = null;
    message.value = null;
    try {
      await api.deleteProfile(id);
      if (selectedId.value === id) {
        selectedId.value = null;
      }
      await load();
      message.value = "已删除配置";
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      mutating.value = false;
    }
  }

  async function apply(id: string): Promise<ApplyReport> {
    applying.value = true;
    error.value = null;
    message.value = null;
    lastReport.value = null;
    try {
      const report = await api.applyProfile(id);
      lastReport.value = report;
      selectedId.value = id;
      const parts = [
        `已应用方案`,
        `部署 ${report.deployed}`,
        `移除 ${report.removed}`,
      ];
      if (report.errors.length > 0) {
        parts.push(`${report.errors.length} 个问题`);
        error.value = report.errors.join("；");
      }
      message.value = parts.join(" · ");
      return report;
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      applying.value = false;
    }
  }

  return {
    profiles,
    selectedId,
    loading,
    applying,
    mutating,
    error,
    message,
    lastReport,
    load,
    create,
    snapshot,
    rename,
    remove,
    apply,
  };
});
