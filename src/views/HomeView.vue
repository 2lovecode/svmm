<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import LayerGuide from "../components/LayerGuide.vue";
import StatusBar from "../components/StatusBar.vue";
import * as api from "../api/tauri";
import { useModsStore } from "../stores/mods";
import { useLibraryStore } from "../stores/library";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";
import { formatAppError, type LibraryMod, type Profile } from "../types/mod";

const mods = useModsStore();
const library = useLibraryStore();
const profiles = useProfilesStore();
const settings = useSettingsStore();

const dragOver = ref(false);
const updatingId = ref<string | null>(null);
const selectValue = ref("");
let unlistenNxm: UnlistenFn | null = null;
let unlistenNxmStarted: UnlistenFn | null = null;

onMounted(async () => {
  try {
    await settings.load();
  } catch {
    /* optional */
  }
  try {
    await profiles.load(settings.settings.lastProfileId);
    selectValue.value = profiles.selectedId ?? "";
  } catch {
    /* profiles may fail before paths are set */
  }
  try {
    await library.loadHome(profiles.selectedId);
  } catch {
    /* shown below */
  }
  try {
    await mods.refreshSmapiStatus();
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
    const unlistenStarted = await listen<{ message?: string }>(
      "nxm-install-started",
      (event) => {
        mods.error = null;
        mods.loading = true;
        mods.statusMessage = event.payload.message ?? "正在通过 NXM 安装…";
      },
    );
    unlistenNxmStarted = unlistenStarted;

    unlistenNxm = await listen<{
      ok: boolean;
      message?: string;
      detail?: string | null;
      mod?: LibraryMod;
    }>("nxm-install-result", async (event) => {
      mods.loading = false;
      const payload = event.payload;
      if (payload.ok) {
        mods.error = null;
        mods.statusMessage = payload.message ?? "已加入本地库";
        try {
          await library.loadHome(library.home?.profile.id ?? profiles.selectedId);
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
  if (unlistenNxmStarted) {
    unlistenNxmStarted();
    unlistenNxmStarted = null;
  }
});

async function onRefreshHome() {
  try {
    await library.loadHome(profiles.selectedId ?? library.home?.profile.id);
  } catch {
    /* shown below */
  }
}

watch(
  () => profiles.selectedId,
  (id) => {
    if (id) selectValue.value = id;
  },
);

async function onProfileChange(event: Event) {
  const id = (event.target as HTMLSelectElement).value;
  if (!id || id === profiles.selectedId) {
    selectValue.value = profiles.selectedId ?? "";
    return;
  }
  const previous = profiles.selectedId ?? "";
  selectValue.value = id;
  profiles.selectedId = id;
  try {
    await library.loadHome(id);
  } catch {
    profiles.selectedId = previous || null;
    selectValue.value = previous;
  }
}

async function onApplyShown() {
  const id = library.home?.profile.id ?? profiles.selectedId ?? "default";
  try {
    await profiles.apply(id);
    await settings.load();
    await library.loadHome(id);
    await mods.refresh();
  } catch {
    /* stores */
  }
}

async function onLaunch() {
  try {
    await mods.launch();
  } catch {
    /* error surfaced in store */
  }
}

function profileName(profile?: Pick<Profile, "id" | "name"> | null): string {
  if (!profile || profile.id === "default" || profile.name.toLowerCase() === "default") {
    return "默认方案";
  }
  return profile.name;
}

async function onUpdateMod(id: string) {
  updatingId.value = id;
  mods.error = null;
  try {
    const result = await api.libraryUpdate(id);
    if (result.browserUrl) {
      try {
        await openUrl(result.browserUrl);
      } catch {
        window.open(result.browserUrl, "_blank", "noopener");
      }
      mods.statusMessage = "已打开 Nexus。在网页上点下载后，会自动更新到本地库。";
      return;
    }
    mods.statusMessage = "已更新";
    await library.loadHome(library.home?.profile.id ?? profiles.selectedId);
  } catch (e) {
    mods.error = formatAppError(e);
    mods.statusMessage = "更新失败";
  } finally {
    updatingId.value = null;
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
    await library.loadHome(library.home?.profile.id ?? profiles.selectedId);
  } catch {
    /* shown in status bar */
  }
}
</script>

<template>
  <div
    class="page-view home-page"
    :class="{ 'drag-over': dragOver }"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <main class="page-body">
      <LayerGuide :step="3" />
      <p v-if="dragOver" class="drop-hint">松开以加入本地库</p>
      <section class="smapi-banner">
        <div class="smapi-banner-copy">
          <label class="profile-select-wrap">
            <span class="sr-only">方案</span>
            <select
              class="profile-select"
              :value="selectValue"
              :disabled="profiles.loading || profiles.applying || library.loading"
              @change="onProfileChange"
            >
              <option v-if="profiles.profiles.length === 0" value="" disabled>暂无方案</option>
              <option v-for="p in profiles.profiles" :key="p.id" :value="p.id">
                {{ profileName(p) }}
              </option>
            </select>
          </label>
          <p>{{ library.home?.applied ? "已应用到游戏目录" : "尚未应用到游戏目录" }}</p>
        </div>
        <div class="smapi-banner-actions">
          <button type="button" class="btn" :disabled="library.loading" @click="onRefreshHome">
            {{ library.loading ? "刷新中…" : "刷新" }}
          </button>
          <button
            v-if="library.home && !library.home.applied"
            type="button"
            class="btn btn-primary"
            :disabled="profiles.applying"
            @click="onApplyShown"
          >
            {{ profiles.applying ? "应用中…" : "应用" }}
          </button>
          <button
            type="button"
            class="btn btn-primary"
            :disabled="mods.launching || mods.smapiInstalled !== true"
            :title="mods.smapiInstalled === false ? '请先在设置里安装 SMAPI' : undefined"
            @click="onLaunch"
          >
            {{ mods.launching ? "启动中…" : "启动" }}
          </button>
        </div>
      </section>
      <ul class="profiles-list home-mod-list">
        <li v-if="library.loading" class="profile-row">加载中…</li>
        <template v-else>
          <li v-if="!library.home || library.home.mods.length === 0" class="profile-row">
            <div class="profile-main">
              <strong>{{ profileName(library.home?.profile) }}还是空的</strong>
              <span class="profile-meta">去浏览页入库，再回到方案里把模组加进来。</span>
            </div>
            <div class="profile-actions">
              <router-link class="btn btn-primary" to="/browse">去浏览</router-link>
              <router-link class="btn btn-ghost" to="/profiles">管理方案</router-link>
            </div>
          </li>
          <li v-for="mod in library.home?.mods ?? []" :key="mod.id" class="profile-row">
            <div class="profile-main">
              <strong>{{ mod.name }}</strong>
              <span class="mod-versions">
                当前 {{ mod.version || "未知" }}
                <template v-if="library.updates[mod.id]"> · 最新 {{ library.updates[mod.id] }}</template>
              </span>
              <span class="profile-meta">{{ mod.author }} · {{ mod.id }}</span>
            </div>
            <div v-if="library.updates[mod.id]" class="profile-actions">
              <button
                type="button"
                class="btn"
                :disabled="updatingId === mod.id"
                @click="onUpdateMod(mod.id)"
              >
                {{ updatingId === mod.id ? "更新中…" : "更新" }}
              </button>
            </div>
          </li>
        </template>
      </ul>
      <p v-if="library.error" class="feedback err">{{ library.error }}</p>
    </main>
    <StatusBar
      :message="mods.statusMessage"
      :enabled-count="library.home?.mods.length ?? 0"
      :total-count="library.home?.mods.length ?? 0"
      :error="mods.error"
    />
  </div>
</template>
