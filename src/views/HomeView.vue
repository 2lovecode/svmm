<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useRouter } from "vue-router";
import TopBar from "../components/TopBar.vue";
import LayerGuide from "../components/LayerGuide.vue";
import StatusBar from "../components/StatusBar.vue";
import { useModsStore } from "../stores/mods";
import { useLibraryStore } from "../stores/library";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";
import type { LibraryMod } from "../types/mod";

const mods = useModsStore();
const library = useLibraryStore();
const profiles = useProfilesStore();
const settings = useSettingsStore();
const router = useRouter();

const dragOver = ref(false);
let unlistenNxm: UnlistenFn | null = null;
let unlistenNxmStarted: UnlistenFn | null = null;

onMounted(async () => {
  try {
    await settings.load();
  } catch {
    /* optional */
  }
  try {
    await library.loadHome();
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
          await library.loadHome();
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

async function onInstallSmapi() {
  try {
    await mods.installSmapi();
  } catch {
    /* shown in status bar */
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
    await library.loadHome();
  } catch {
    /* shown in status bar */
  }
}
</script>

<template>
  <div
    class="page home-page"
    :class="{ 'drag-over': dragOver }"
    @dragover="onDragOver"
    @dragleave="onDragLeave"
    @drop="onDrop"
  >
    <TopBar
      :busy="
        mods.loading ||
        mods.checkingUpdates ||
        mods.launching ||
        mods.installingSmapi ||
        !!mods.togglingPath ||
        !!mods.actionPath ||
        profiles.applying
      "
    />
    <main class="page-body">
      <LayerGuide :step="3" />
      <section v-if="mods.smapiInstalled === false || mods.installingSmapi" class="smapi-banner">
        <div class="smapi-banner-copy">
          <p v-if="mods.installingSmapi">
            {{ mods.smapiProgress?.message ?? "正在安装 SMAPI…" }}
          </p>
          <p v-else-if="mods.gameFound">尚未安装 SMAPI。安装后才能运行模组。</p>
          <p v-else>未找到星露谷物语。请先在设置里指定游戏目录，再安装 SMAPI。</p>
          <div
            v-if="mods.installingSmapi"
            class="progress-track"
            :class="{ indeterminate: mods.smapiProgress?.percent == null }"
          >
            <span :style="{ width: `${mods.smapiProgress?.percent ?? 40}%` }" />
          </div>
        </div>
        <div class="smapi-banner-actions">
          <button
            v-if="mods.gameFound"
            type="button"
            class="btn btn-primary"
            :disabled="mods.installingSmapi"
            @click="onInstallSmapi"
          >
            {{ mods.installingSmapi ? "安装中…" : "安装 SMAPI" }}
          </button>
          <button v-else type="button" class="btn" @click="router.push('/settings')">
            打开设置
          </button>
        </div>
      </section>
      <p v-if="dragOver" class="drop-hint">松开以加入本地库</p>
      <section class="smapi-banner">
        <div class="smapi-banner-copy">
          <p>
            默认方案「{{ library.home?.profile.name ?? "默认方案" }}」
            ·
            {{ library.home?.applied ? "已应用到游戏目录" : "尚未应用到游戏目录" }}
          </p>
        </div>
        <div class="smapi-banner-actions">
          <button
            type="button"
            class="btn btn-primary"
            :disabled="profiles.applying"
            @click="profiles.apply('default').then(() => library.loadHome())"
          >
            应用
          </button>
        </div>
      </section>
      <ul class="profiles-list">
        <li v-if="library.loading">加载中…</li>
        <li v-else-if="!library.home || library.home.mods.length === 0" class="profile-row">
          <div class="profile-main">
            <strong>默认方案还是空的</strong>
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
            <span class="profile-meta">{{ mod.version }} · {{ mod.author }} · {{ mod.id }}</span>
          </div>
        </li>
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
