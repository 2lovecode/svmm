<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useModsStore } from "../stores/mods";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";

defineProps<{
  busy?: boolean;
}>();

const router = useRouter();
const mods = useModsStore();
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
    await mods.refresh();
  } catch {
    /* error surfaced in store */
  }
}

async function onCheckUpdates() {
  try {
    await mods.checkUpdates();
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
    await mods.refresh();
  } catch {
    selectValue.value = previous;
  }
}
</script>

<template>
  <header class="topbar">
    <div class="brand">
      <span class="brand-mark" aria-hidden="true" />
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
            暂无配置
          </option>
          <option v-for="p in profiles.profiles" :key="p.id" :value="p.id">
            {{ p.name }}
          </option>
        </select>
      </label>
      <button type="button" class="btn btn-ghost" @click="router.push('/profiles')">
        配置
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
        class="btn"
        :disabled="busy || mods.checkingUpdates || mods.loading"
        @click="onCheckUpdates"
      >
        {{ mods.checkingUpdates ? "检查中…" : "检查更新" }}
      </button>
      <button
        type="button"
        class="btn btn-primary"
        :disabled="busy || mods.launching"
        @click="onLaunch"
      >
        {{ mods.launching ? "启动中…" : "启动 SMAPI" }}
      </button>
      <button type="button" class="btn btn-ghost" @click="router.push('/settings')">
        设置
      </button>
    </nav>
  </header>
</template>
