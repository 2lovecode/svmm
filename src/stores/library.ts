import { defineStore } from "pinia";
import { ref } from "vue";
import * as api from "../api/tauri";
import { formatAppError, type LibraryMod, type ProfileState } from "../types/mod";

export const useLibraryStore = defineStore("library", () => {
  const home = ref<ProfileState | null>(null);
  const mods = ref<LibraryMod[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const message = ref<string | null>(null);

  async function loadHome() {
    loading.value = true;
    error.value = null;
    try {
      home.value = await api.homeState();
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
      message.value = "已从本地库删除";
      await load();
    } catch (e) {
      error.value = formatAppError(e);
      throw e;
    }
  }

  return { home, mods, loading, error, message, loadHome, load, remove };
});
