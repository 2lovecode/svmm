<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import LayerGuide from "../components/LayerGuide.vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import * as api from "../api/tauri";
import {
  formatAppError,
  type NexusBrowsePage,
  type NexusCatalogMod,
  type NexusCategory,
} from "../types/mod";

const categories = ref<NexusCategory[]>([]);
const result = ref<NexusBrowsePage | null>(null);
const category = ref("");
const keyword = ref("");
const modId = ref("");
const addingId = ref<number | null>(null);
const notice = ref<string | null>(null);
const currentPage = ref(1);
const loading = ref(false);
const error = ref<string | null>(null);
let unlistenNxm: UnlistenFn | null = null;

const pageCount = computed(() => {
  const total = result.value?.total ?? 0;
  const size = result.value?.pageSize || 20;
  return Math.max(1, Math.ceil(total / size));
});

const rangeLabel = computed(() => {
  const total = result.value?.total ?? 0;
  if (total === 0) return "0 个";
  const size = result.value?.pageSize || 20;
  const start = (currentPage.value - 1) * size + 1;
  const end = Math.min(currentPage.value * size, total);
  return `${start.toLocaleString("zh-CN")}–${end.toLocaleString("zh-CN")} / ${total.toLocaleString("zh-CN")}`;
});

onMounted(async () => {
  try {
    categories.value = await api.nexusCategories();
  } catch (e) {
    error.value = formatAppError(e);
  }
  await load(1);
  try {
    unlistenNxm = await listen<{ ok: boolean; message?: string }>(
      "nxm-install-result",
      async (event) => {
        if (event.payload.ok) {
          notice.value = "已入库到本地库";
          await load(currentPage.value);
        } else if (event.payload.message) {
          error.value = event.payload.message;
        }
      },
    );
  } catch {
    /* browser preview has no Tauri events */
  }
});

onUnmounted(() => {
  unlistenNxm?.();
});

async function load(page: number) {
  loading.value = true;
  error.value = null;
  try {
    currentPage.value = page;
    const parsed = Number(modId.value.trim());
    const parsedId = modId.value.trim() && Number.isFinite(parsed) ? parsed : null;
    result.value = await api.nexusBrowseMods(
      page,
      category.value || null,
      keyword.value.trim() || null,
      parsedId,
    );
  } catch (e) {
    error.value = formatAppError(e);
  } finally {
    loading.value = false;
  }
}

function onCategoryChange(event: Event) {
  category.value = (event.target as HTMLSelectElement).value;
  void load(1);
}

async function openMod(mod: NexusCatalogMod) {
  const url = `https://www.nexusmods.com/stardewvalley/mods/${mod.modId}`;
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener");
  }
}

function statusLabel(status: string): string {
  if (status === "owned") return "已有";
  if (status === "update") return "有更新";
  return "未下载";
}

async function addToLibrary(mod: NexusCatalogMod) {
  addingId.value = mod.modId;
  error.value = null;
  notice.value = null;
  try {
    const imported = await api.libraryAddFromNexus(mod.modId, mod.category || null);
    if (imported.browserUrl) {
      try {
        await openUrl(imported.browserUrl);
      } catch {
        window.open(imported.browserUrl, "_blank", "noopener");
      }
      notice.value = "已打开 Nexus。在网页上点下载后，会自动入库到本地库。";
      return;
    }
    notice.value = "已入库";
    await load(currentPage.value);
  } catch (e) {
    error.value = formatAppError(e);
  } finally {
    addingId.value = null;
  }
}

function formatCount(value: number): string {
  return value.toLocaleString("zh-CN");
}
</script>

<template>
  <div class="page-view">
    <header class="settings-header">
      <h1>Nexus 模组</h1>
    </header>
    <main class="settings-body browse-body">
      <LayerGuide :step="1" />
      <div class="mod-toolbar">
        <label class="field-inline">
          <span>关键词</span>
          <input v-model="keyword" type="text" placeholder="名称" @keydown.enter="load(1)" />
        </label>
        <label class="field-inline">
          <span>模组 ID</span>
          <input v-model="modId" type="text" placeholder="例如 2400" @keydown.enter="load(1)" />
        </label>
        <button type="button" class="btn" :disabled="loading" @click="load(1)">查询</button>
        <label class="field-inline">
          <span>分类</span>
          <select :value="category" :disabled="loading" @change="onCategoryChange">
            <option value="">全部</option>
            <option v-for="item in categories" :key="item.name" :value="item.name">
              {{ item.name }}（{{ formatCount(item.count) }}）
            </option>
          </select>
        </label>
        <div class="pager">
          <span class="pager-range">{{ rangeLabel }}</span>
          <button
            type="button"
            class="btn"
            :disabled="loading || currentPage <= 1"
            @click="load(currentPage - 1)"
          >
            上一页
          </button>
          <span class="pager-index">{{ currentPage }} / {{ pageCount }}</span>
          <button
            type="button"
            class="btn"
            :disabled="loading || currentPage >= pageCount"
            @click="load(currentPage + 1)"
          >
            下一页
          </button>
        </div>
      </div>
      <p v-if="notice" class="feedback ok">{{ notice }}</p>
      <p v-if="error" class="feedback err">{{ error }}</p>
      <p v-else-if="loading && !result" class="hint">正在加载…</p>
      <ul v-else class="catalog-list">
        <li v-if="result && result.mods.length === 0" class="empty">没有符合筛选的模组。</li>
        <li v-for="mod in result?.mods ?? []" :key="mod.modId" class="catalog-card">
          <img
            v-if="mod.thumbnailUrl"
            class="catalog-thumb"
            :src="mod.thumbnailUrl"
            alt=""
          />
          <div class="catalog-copy">
            <div class="catalog-title">
              <strong>{{ mod.name || "（未命名）" }}</strong>
              <span class="badge">{{ statusLabel(mod.libraryStatus) }}</span>
              <span class="badge">{{ mod.category || "未分类" }}</span>
            </div>
            <p class="catalog-meta">
              {{ mod.author || "未知作者" }}
              <template v-if="mod.version"> · {{ mod.version }}</template>
              · 下载 {{ formatCount(mod.downloads) }}
              · 推荐 {{ formatCount(mod.endorsements) }}
            </p>
            <p v-if="mod.summary" class="catalog-summary">{{ mod.summary }}</p>
          </div>
          <button
            type="button"
            class="btn btn-primary"
            :disabled="addingId === mod.modId || mod.libraryStatus === 'owned'"
            @click="addToLibrary(mod)"
          >
            {{ mod.libraryStatus === "owned" ? "已有" : addingId === mod.modId ? "入库中…" : "入库" }}
          </button>
          <button type="button" class="btn" @click="openMod(mod)">打开</button>
        </li>
      </ul>
    </main>
  </div>
</template>
