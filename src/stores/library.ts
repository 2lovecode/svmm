import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "../api/tauri";
import { formatAppError, type LibraryMod, type ProfileState, type UpdateInfo } from "../types/mod";

export const useLibraryStore = defineStore("library", () => {
  const home = ref<ProfileState | null>(null);
  const mods = ref<LibraryMod[]>([]);
  const updates = ref<Record<string, string>>({});
  const loading = ref(false);
  const error = ref<string | null>(null);
  const message = ref<string | null>(null);

  async function loadUpdates() {
    try {
      const infos: UpdateInfo[] = await api.libraryCheckUpdates();
      const next: Record<string, string> = {};
      for (const info of infos) {
        if (
          info.suggestedVersion &&
          (info.status === "update_available" || info.status === "unofficial_update")
        ) {
          next[info.id] = info.suggestedVersion;
        }
      }
      updates.value = next;
    } catch {
      updates.value = {};
    }
  }

  async function loadHome(profileId?: string | null) {
    loading.value = true;
    error.value = null;
    try {
      home.value = profileId
        ? await api.profileDetail(profileId)
        : await api.homeState();
      void loadUpdates();
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function load(filter?: { category?: string; id?: string; keyword?: string }) {
    loading.value = true;
    error.value = null;
    try {
      mods.value = await api.listLibrary({
        category: filter?.category || null,
        id: filter?.id || null,
        keyword: filter?.keyword || null,
      });
      void loadUpdates();
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function remove(id: string) {
    error.value = null;
    message.value = null;
    try {
      await api.deleteLibraryMod(id);
      message.value = "已从本地库删除，并移出所有方案";
      await load();
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    }
  }

  return { home, mods, updates, loading, error, message, loadHome, load, remove };
});
