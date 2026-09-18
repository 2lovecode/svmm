<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import LayerGuide from "../components/LayerGuide.vue";
import * as api from "../api/tauri";
import { useModsStore } from "../stores/mods";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";
import { formatAppError, type LibraryMod, type Profile, type ProfileState } from "../types/mod";

const router = useRouter();
const profiles = useProfilesStore();
const mods = useModsStore();
const settings = useSettingsStore();

const newName = ref("");
const renamingId = ref<string | null>(null);
const renameDraft = ref("");
const detail = ref<ProfileState | null>(null);
const libraryMods = ref<LibraryMod[]>([]);
const pickId = ref("");

onMounted(async () => {
  try {
    await settings.load();
  } catch {
    /* optional for preferred id */
  }
  try {
    await profiles.load(settings.settings.lastProfileId);
    libraryMods.value = await api.listLibrary();
    if (profiles.selectedId) {
      detail.value = await api.profileDetail(profiles.selectedId);
    }
  } catch {
    /* error in store */
  }
});

async function onCreate() {
  const name = newName.value.trim();
  if (!name) return;
  try {
    await profiles.create(name);
    newName.value = "";
  } catch {
    /* store error */
  }
}

function startRename(profile: Profile) {
  renamingId.value = profile.id;
  renameDraft.value = profile.name;
}

function cancelRename() {
  renamingId.value = null;
  renameDraft.value = "";
}

async function confirmRename() {
  const id = renamingId.value;
  const name = renameDraft.value.trim();
  if (!id || !name) return;
  try {
    await profiles.rename(id, name);
    cancelRename();
  } catch {
    /* store error */
  }
}

async function onDelete(profile: Profile) {
  if (profile.id === "default") {
    window.alert("不能删除默认配置");
    return;
  }
  const ok = window.confirm(`确定删除配置「${profile.name}」？此操作不可撤销。`);
  if (!ok) return;
  try {
    await profiles.remove(profile.id);
  } catch {
    /* store error */
  }
}

async function onApply(profile: Profile) {
  try {
    await profiles.apply(profile.id);
    await settings.load();
    detail.value = await api.profileDetail(profile.id);
    await mods.refresh();
  } catch {
    /* store / mods error */
  }
}

async function openDetail(profile: Profile) {
  profiles.selectedId = profile.id;
  try {
    detail.value = await api.profileDetail(profile.id);
  } catch (e) {
    profiles.error = formatAppError(e);
  }
}

async function onAddMod() {
  if (!detail.value || !pickId.value) return;
  try {
    await api.addProfileMod(detail.value.profile.id, pickId.value);
    detail.value = await api.profileDetail(detail.value.profile.id);
    await profiles.load(detail.value.profile.id);
    pickId.value = "";
  } catch (e) {
    profiles.error = formatAppError(e);
  }
}

async function onRemoveMod(modId: string) {
  if (!detail.value) return;
  try {
    await api.removeProfileMod(detail.value.profile.id, modId);
    detail.value = await api.profileDetail(detail.value.profile.id);
    await profiles.load(detail.value.profile.id);
  } catch (e) {
    profiles.error = formatAppError(e);
  }
}
</script>

<template>
  <div class="page settings-page">
    <header class="settings-header">
      <button type="button" class="btn btn-ghost" @click="router.push('/')">
        ← 返回
      </button>
      <h1>方案</h1>
    </header>

    <main class="settings-body profiles-body">
      <LayerGuide :step="3" />

      <section class="profiles-section">
        <h2>新建</h2>
        <div class="field-row">
          <input
            v-model="newName"
            type="text"
            placeholder="配置名称"
            :disabled="profiles.mutating"
            @keydown.enter="onCreate"
          />
          <button
            type="button"
            class="btn btn-primary"
            :disabled="profiles.mutating || !newName.trim()"
            @click="onCreate"
          >
            创建
          </button>
        </div>
        <p class="field-note">新建的是空方案，再从本地库把模组加进来。</p>
      </section>

      <section class="profiles-section">
        <h2>已有方案</h2>
        <p v-if="profiles.loading" class="hint">加载中…</p>
        <ul v-else-if="profiles.profiles.length === 0" class="profiles-list empty-list">
          <li>暂无配置</li>
        </ul>
        <ul v-else class="profiles-list">
          <li v-for="p in profiles.profiles" :key="p.id" class="profile-row">
            <div class="profile-main">
              <template v-if="renamingId === p.id">
                <input
                  v-model="renameDraft"
                  type="text"
                  class="rename-input"
                  :disabled="profiles.mutating"
                  @keydown.enter="confirmRename"
                  @keydown.escape="cancelRename"
                />
              </template>
              <template v-else>
                <strong>{{ p.name }}</strong>
                <span class="profile-meta">
                  {{ p.modIds.length }} 个模组 · {{ p.id }}
                </span>
              </template>
            </div>
            <div class="profile-actions">
              <template v-if="renamingId === p.id">
                <button
                  type="button"
                  class="btn btn-primary"
                  :disabled="profiles.mutating || !renameDraft.trim()"
                  @click="confirmRename"
                >
                  保存
                </button>
                <button type="button" class="btn btn-ghost" @click="cancelRename">
                  取消
                </button>
              </template>
              <template v-else>
                <button type="button" class="btn" @click="openDetail(p)">管理</button>
                <button
                  type="button"
                  class="btn btn-primary"
                  :disabled="profiles.applying || profiles.mutating"
                  @click="onApply(p)"
                >
                  应用
                </button>
                <button
                  type="button"
                  class="btn"
                  :disabled="profiles.mutating"
                  @click="startRename(p)"
                >
                  重命名
                </button>
                <button
                  type="button"
                  class="btn btn-danger"
                  :disabled="profiles.mutating || p.id === 'default'"
                  :title="p.id === 'default' ? '不能删除默认配置' : undefined"
                  @click="onDelete(p)"
                >
                  删除
                </button>
              </template>
            </div>
          </li>
        </ul>
      </section>

      <section v-if="detail" class="profiles-section">
        <h2>{{ detail.profile.name }} 的模组</h2>
        <p class="hint">{{ detail.applied ? "已应用到游戏目录" : "尚未应用到游戏目录" }}</p>
        <ul class="profiles-list">
          <li v-if="detail.mods.length === 0">还没有模组。</li>
          <li v-for="mod in detail.mods" :key="mod.id" class="profile-row">
            <div class="profile-main">
              <strong>{{ mod.name }}</strong>
              <span class="profile-meta">{{ mod.version }} · {{ mod.id }}</span>
            </div>
            <button type="button" class="btn" @click="onRemoveMod(mod.id)">移出</button>
          </li>
        </ul>
        <div class="field-row">
          <select v-model="pickId">
            <option value="">从本地库添加</option>
            <option v-for="mod in libraryMods" :key="mod.id" :value="mod.id">
              {{ mod.name }}（{{ mod.version }}）
            </option>
          </select>
          <button type="button" class="btn btn-primary" :disabled="!pickId" @click="onAddMod">
            添加
          </button>
        </div>
      </section>

      <p v-if="profiles.message" class="feedback ok">{{ profiles.message }}</p>
      <p v-if="profiles.error" class="feedback err">{{ profiles.error }}</p>
    </main>
  </div>
</template>
