<script setup lang="ts">
import { useRouter } from "vue-router";
import { useModsStore } from "../stores/mods";

defineProps<{
  busy?: boolean;
}>();

const router = useRouter();
const mods = useModsStore();

async function onRefresh() {
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
