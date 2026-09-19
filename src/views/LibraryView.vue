<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import LayerGuide from "../components/LayerGuide.vue";
import * as api from "../api/tauri";
import { useLibraryStore } from "../stores/library";
import { useProfilesStore } from "../stores/profiles";
import { formatAppError, type Profile } from "../types/mod";

const library = useLibraryStore();
const profiles = useProfilesStore();
const query = ref("");
const busyId = ref<string | null>(null);
const selected = ref<string[]>([]);
const pickerOpen = ref(false);
const adding = ref(false);
let unlistenNxm: UnlistenFn | null = null;

onMounted(() => {
  void profiles.load().catch(() => {
    /* shown if the add action is used */
  });
  void reload();
  void listen<{ ok: boolean; message?: string }>("nxm-install-result", async (event) => {
    if (event.payload.ok) {
      library.message = event.payload.message ?? "已更新到本地库";
      await reload();
    } else if (event.payload.message) {
      library.error = event.payload.message;
    }
  }).then((unlisten) => {
    unlistenNxm = unlisten;
  });
});

onUnmounted(() => {
  unlistenNxm?.();
  unlistenNxm = null;
});

function profileLabel(profile: Pick<Profile, "id" | "name">): string {
  if (profile.id === "default" || profile.name.toLowerCase() === "default") {
    return "默认方案";
  }
  return profile.name;
}

function isSelected(id: string): boolean {
  return selected.value.includes(id);
}

function allVisibleSelected(): boolean {
  return library.mods.length > 0 && library.mods.every((mod) => selected.value.includes(mod.id));
}

function onToggle(id: string, event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  if (checked) {
    if (!selected.value.includes(id)) selected.value = [...selected.value, id];
  } else {
    selected.value = selected.value.filter((item) => item !== id);
  }
}

function onToggleAll(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  const visible = new Set(library.mods.map((mod) => mod.id));
  if (checked) {
    const next = new Set(selected.value);
    for (const id of visible) next.add(id);
    selected.value = [...next];
  } else {
    selected.value = selected.value.filter((id) => !visible.has(id));
  }
}

function openPicker() {
  if (selected.value.length === 0) return;
  library.error = null;
  pickerOpen.value = true;
}

async function confirmAdd(profile: Profile) {
  adding.value = true;
  library.error = null;
  try {
    for (const id of selected.value) {
      await api.addProfileMod(profile.id, id);
    }
    library.message = `已把 ${selected.value.length} 个模组加入「${profileLabel(profile)}」。要在游戏里用，请再应用该方案。`;
    selected.value = [];
    pickerOpen.value = false;
  } catch (e) {
    library.error = formatAppError(e);
  } finally {
    adding.value = false;
  }
}

async function reload() {
  await library.load({
    keyword: query.value.trim(),
  });
}

async function onUpdate(id: string) {
  busyId.value = id;
  library.error = null;
  try {
    const result = await api.libraryUpdate(id);
    if (result.browserUrl) {
      try {
        await openUrl(result.browserUrl);
      } catch {
        window.open(result.browserUrl, "_blank", "noopener");
      }
      library.message = "已打开 Nexus。在网页上点下载后，会自动更新到本地库。";
      return;
    }
    library.message = "已更新，包含该模组的方案使用这一份";
    await reload();
  } catch (e) {
    library.error = formatAppError(e);
  } finally {
    busyId.value = null;
  }
}

async function onDelete(id: string, name: string) {
  if (!window.confirm(`从本地库删除「${name}」？包含它的方案也会移出。玩家配置会保留。`)) return;
  busyId.value = id;
  try {
    await library.remove(id);
  } catch {
    /* store */
  } finally {
    busyId.value = null;
  }
}
</script>

<template>
  <div class="page-view">
    <header class="settings-header">
      <h1>本地库</h1>
    </header>
    <main class="settings-body library-body">
      <LayerGuide :step="2" />
      <div class="field-row">
        <input
          v-model="query"
          type="text"
          placeholder="名称、作者或 UniqueID"
          @keydown.enter="reload"
        />
        <button type="button" class="btn" :disabled="library.loading" @click="reload">筛选</button>
        <button type="button" class="btn" :disabled="library.loading" @click="reload">刷新</button>
        <button
          type="button"
          class="btn btn-primary"
          :disabled="selected.length === 0 || adding"
          @click="openPicker"
        >
          添加至方案{{ selected.length ? `（${selected.length}）` : "" }}
        </button>
      </div>
      <p v-if="library.loading" class="hint">加载中…</p>
      <ul v-else class="profiles-list library-list">
        <li v-if="library.mods.length === 0">本地库是空的。</li>
        <li v-else class="profile-row select-all-row">
          <label class="mod-check">
            <input type="checkbox" :checked="allVisibleSelected()" @change="onToggleAll" />
            <span>全选</span>
          </label>
          <span v-if="selected.length" class="profile-meta">已选 {{ selected.length }}</span>
        </li>
        <li v-for="mod in library.mods" :key="mod.id" class="profile-row">
          <label class="mod-check">
            <input type="checkbox" :checked="isSelected(mod.id)" @change="onToggle(mod.id, $event)" />
            <span class="sr-only">选择 {{ mod.name }}</span>
          </label>
          <div class="profile-main">
            <strong>{{ mod.name }}</strong>
            <span class="mod-versions">
              当前 {{ mod.version || "未知" }}
              <template v-if="library.updates[mod.id]"> · 最新 {{ library.updates[mod.id] }}</template>
            </span>
            <span class="profile-meta">
              {{ mod.author || "未知作者" }} · {{ mod.id }}
              <template v-if="mod.category"> · {{ mod.category }}</template>
              <template v-if="!mod.idFromManifest"> · 无 UniqueID</template>
            </span>
          </div>
          <div class="profile-actions">
            <button
              v-if="library.updates[mod.id]"
              type="button"
              class="btn"
              :disabled="busyId === mod.id"
              @click="onUpdate(mod.id)"
            >
              更新
            </button>
            <button type="button" class="btn btn-danger" :disabled="busyId === mod.id" @click="onDelete(mod.id, mod.name)">
              删除
            </button>
          </div>
        </li>
      </ul>
      <p v-if="library.message" class="feedback ok">{{ library.message }}</p>
      <p v-if="library.error" class="feedback err">{{ library.error }}</p>
    </main>
    <div v-if="pickerOpen" class="modal-backdrop" @click.self="pickerOpen = false">
      <section class="modal-card" role="dialog" aria-modal="true" aria-labelledby="profile-picker-title">
        <h2 id="profile-picker-title">添加到方案</h2>
        <p class="hint">已选 {{ selected.length }} 个模组。选择一个方案加入，不会改游戏目录。</p>
        <ul v-if="profiles.profiles.length > 0" class="profiles-list">
          <li v-for="profile in profiles.profiles" :key="profile.id">
            <button
              type="button"
              class="btn profile-picker-item"
              :disabled="adding"
              @click="confirmAdd(profile)"
            >
              {{ profileLabel(profile) }}
            </button>
          </li>
        </ul>
        <p v-else class="hint">还没有方案。</p>
        <div class="profile-actions">
          <button type="button" class="btn btn-ghost" :disabled="adding" @click="pickerOpen = false">
            取消
          </button>
        </div>
      </section>
    </div>
  </div>
</template>
