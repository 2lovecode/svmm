<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useModsStore } from "../stores/mods";
import { useLibraryStore } from "../stores/library";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";

defineProps<{
  busy?: boolean;
}>();

const router = useRouter();
const mods = useModsStore();
const library = useLibraryStore();
const profiles = useProfilesStore();
const settings = useSettingsStore();

const selectValue = ref("");

onMounted(async () => {
  try {
    await settings.load();
  } catch {
    /* optional */
  }
  try {
    await mods.refreshSmapiStatus();
  } catch {
    /* optional */
  }
  try {
    await profiles.load(settings.settings.lastProfileId);
    selectValue.value = profiles.selectedId ?? "";
  } catch {
    /* profiles may fail before paths are set */
  }
});

watch(
  () => profiles.selectedId,
  (id) => {
    if (id) selectValue.value = id;
  },
);

async function onRefresh() {
  try {
    await library.loadHome();
  } catch {
    /* home may be unavailable before paths exist */
  }
  try {
    await mods.refresh();
  } catch {
    /* error surfaced in store */
  }
}

async function onLaunch() {
  try {
    await mods.launch();
  } catch {
    /* error surfaced in store */
  }
}

async function onProfileChange(event: Event) {
  const id = (event.target as HTMLSelectElement).value;
  if (!id || id === profiles.selectedId) {
    selectValue.value = profiles.selectedId ?? "";
    return;
  }
  const previous = profiles.selectedId ?? "";
  selectValue.value = id;
  try {
    await profiles.apply(id);
    await settings.load();
    await library.loadHome();
    await mods.refresh();
  } catch {
    selectValue.value = previous;
  }
}
</script>

<template>
  <header class="topbar">
    <div class="brand">
      <svg class="junimo" viewBox="0 0 8 8" aria-hidden="true">
        <g shape-rendering="crispEdges">
          <rect x="2" y="0" width="4" height="1" fill="#4c6840" />
          <rect x="1" y="1" width="6" height="1" fill="#4c6840" />
          <rect x="1" y="2" width="6" height="1" fill="#4c6840" />
          <rect x="1" y="3" width="6" height="1" fill="#4c6840" />
          <rect x="1" y="4" width="6" height="1" fill="#5c7850" />
          <rect x="1" y="5" width="6" height="1" fill="#4c6840" />
          <rect x="2" y="6" width="1" height="1" fill="#4c6840" />
          <rect x="5" y="6" width="1" height="1" fill="#4c6840" />
          <rect x="2" y="2" width="1" height="1" fill="#f7f1df" />
          <rect x="5" y="2" width="1" height="1" fill="#f7f1df" />
          <rect x="2" y="3" width="1" height="1" fill="#1c140e" />
          <rect x="5" y="3" width="1" height="1" fill="#1c140e" />
        </g>
      </svg>
      <div class="brand-text">
        <strong>SVMM</strong>
        <span class="brand-sub">星露谷模组管理</span>
      </div>
    </div>
    <nav class="actions">
      <label class="profile-select-wrap">
        <span class="sr-only">配置方案</span>
        <select
          class="profile-select"
          :value="selectValue"
          :disabled="busy || profiles.loading || profiles.applying || mods.loading"
          @change="onProfileChange"
        >
          <option v-if="profiles.profiles.length === 0" value="" disabled>
            暂无方案
          </option>
          <option v-for="p in profiles.profiles" :key="p.id" :value="p.id">
            {{ p.name }}
          </option>
        </select>
      </label>
      <button type="button" class="btn btn-ghost" @click="router.push('/browse')">
        浏览
      </button>
      <button type="button" class="btn btn-ghost" @click="router.push('/library')">
        本地库
      </button>
      <button type="button" class="btn btn-ghost" @click="router.push('/profiles')">
        方案
      </button>
      <button
        type="button"
        class="btn"
        :disabled="busy || mods.loading"
        @click="onRefresh"
      >
        {{ mods.loading ? "刷新中…" : "刷新" }}
      </button>
      <button
        type="button"
        class="btn btn-primary"
        :disabled="busy || mods.launching || mods.smapiInstalled !== true"
        :title="mods.smapiInstalled === false ? '请先在首页安装 SMAPI' : undefined"
        @click="onLaunch"
      >
        {{ mods.launching ? "启动中…" : "启动" }}
      </button>
      <button type="button" class="btn btn-ghost" @click="router.push('/settings')">
        设置
      </button>
    </nav>
  </header>
</template>
