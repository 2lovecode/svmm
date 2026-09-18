<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import LayerGuide from "../components/LayerGuide.vue";
import * as api from "../api/tauri";
import { useLibraryStore } from "../stores/library";
import { formatAppError, type NexusFileInfo } from "../types/mod";

const router = useRouter();
const library = useLibraryStore();
const category = ref("");
const modId = ref("");
const keyword = ref("");
const busyId = ref<string | null>(null);
const filesFor = ref<string | null>(null);
const files = ref<NexusFileInfo[]>([]);
const pickedFile = ref<number | null>(null);

onMounted(() => {
  void reload();
});

async function reload() {
  await library.load({
    category: category.value.trim(),
    id: modId.value.trim(),
    keyword: keyword.value.trim(),
  });
}

async function onUpdate(id: string) {
  busyId.value = id;
  library.error = null;
  try {
    await api.libraryUpdate(id);
    library.message = "已更新，包含该模组的方案使用这一份";
    await reload();
  } catch (e) {
    library.error = formatAppError(e);
  } finally {
    busyId.value = null;
  }
}

async function onShowFiles(id: string) {
  busyId.value = id;
  library.error = null;
  try {
    files.value = await api.libraryNexusFiles(id);
    filesFor.value = id;
    pickedFile.value = files.value[0]?.fileId ?? null;
  } catch (e) {
    library.error = formatAppError(e);
  } finally {
    busyId.value = null;
  }
}

async function onDowngrade() {
  if (!filesFor.value || pickedFile.value == null) return;
  busyId.value = filesFor.value;
  try {
    await api.libraryDowngrade(filesFor.value, pickedFile.value);
    library.message = "已换成所选版本";
    filesFor.value = null;
    await reload();
  } catch (e) {
    library.error = formatAppError(e);
  } finally {
    busyId.value = null;
  }
}

async function onDelete(id: string, name: string) {
  if (!window.confirm(`从本地库删除「${name}」？玩家配置会保留。`)) return;
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
  <div class="page settings-page">
    <header class="settings-header">
      <button type="button" class="btn btn-ghost" @click="router.push('/')">← 返回</button>
      <h1>本地库</h1>
    </header>
    <main class="settings-body">
      <LayerGuide :step="2" />
      <div class="field-row">
        <input v-model="category" type="text" placeholder="分类" @keydown.enter="reload" />
        <input v-model="modId" type="text" placeholder="UniqueID" @keydown.enter="reload" />
        <input v-model="keyword" type="text" placeholder="关键词" @keydown.enter="reload" />
        <button type="button" class="btn" :disabled="library.loading" @click="reload">筛选</button>
      </div>
      <p v-if="library.loading" class="hint">加载中…</p>
      <ul v-else class="profiles-list">
        <li v-if="library.mods.length === 0">本地库是空的。</li>
        <li v-for="mod in library.mods" :key="mod.id" class="profile-row">
          <div class="profile-main">
            <strong>{{ mod.name }}</strong>
            <span class="profile-meta">
              {{ mod.version }} · {{ mod.author || "未知作者" }} · {{ mod.id }}
              <template v-if="mod.category"> · {{ mod.category }}</template>
              <template v-if="!mod.idFromManifest"> · 无 UniqueID</template>
            </span>
          </div>
          <div class="profile-actions">
            <button
              type="button"
              class="btn"
              :disabled="busyId === mod.id || mod.nexusModId == null"
              @click="onUpdate(mod.id)"
            >
              更新
            </button>
            <button
              type="button"
              class="btn"
              :disabled="busyId === mod.id || mod.nexusModId == null"
              @click="onShowFiles(mod.id)"
            >
              降级
            </button>
            <button type="button" class="btn btn-danger" :disabled="busyId === mod.id" @click="onDelete(mod.id, mod.name)">
              删除
            </button>
          </div>
        </li>
      </ul>
      <section v-if="filesFor" class="profiles-section">
        <h2>选择历史文件</h2>
        <div class="field-row">
          <select v-model.number="pickedFile">
            <option v-for="file in files" :key="file.fileId" :value="file.fileId">
              {{ file.version || file.name || file.fileId }} · {{ file.categoryName || "文件" }}
            </option>
          </select>
          <button type="button" class="btn btn-primary" :disabled="pickedFile == null" @click="onDowngrade">
            换成这一版
          </button>
        </div>
      </section>
      <p v-if="library.message" class="feedback ok">{{ library.message }}</p>
      <p v-if="library.error" class="feedback err">{{ library.error }}</p>
    </main>
  </div>
</template>
