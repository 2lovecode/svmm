<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useModsStore } from "../stores/mods";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";
import type { Profile } from "../types/mod";

const router = useRouter();
const profiles = useProfilesStore();
const mods = useModsStore();
const settings = useSettingsStore();

const newName = ref("");
const snapshotName = ref("");
const renamingId = ref<string | null>(null);
const renameDraft = ref("");

onMounted(async () => {
  try {
    await settings.load();
  } catch {
    /* optional for preferred id */
  }
  try {
    await profiles.load(settings.settings.lastProfileId);
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

async function onSnapshot() {
  const name = snapshotName.value.trim();
  if (!name) return;
  try {
    await profiles.snapshot(name);
    snapshotName.value = "";
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
    await mods.refresh();
  } catch {
    /* store / mods error */
  }
}
</script>

<template>
  <div class="page settings-page">
    <header class="settings-header">
      <button type="button" class="btn btn-ghost" @click="router.push('/')">
        ← 返回
      </button>
      <h1>模组配置</h1>
    </header>

    <main class="settings-body profiles-body">
      <p class="hint">
        配置保存一组启用模组（按 UniqueID）。应用后会按差异启用/禁用对应模组。
      </p>

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
        <p class="field-note">创建时会快照当前已启用的模组列表。</p>
      </section>

      <section class="profiles-section">
        <h2>从当前状态快照</h2>
        <div class="field-row">
          <input
            v-model="snapshotName"
            type="text"
            placeholder="快照名称"
            :disabled="profiles.mutating"
            @keydown.enter="onSnapshot"
          />
          <button
            type="button"
            class="btn"
            :disabled="profiles.mutating || !snapshotName.trim()"
            @click="onSnapshot"
          >
            快照
          </button>
        </div>
      </section>

      <section class="profiles-section">
        <h2>已有配置</h2>
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
                  {{ p.enabledModIds.length }} 个模组 · {{ p.id }}
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

      <p v-if="profiles.message" class="feedback ok">{{ profiles.message }}</p>
      <p v-if="profiles.error" class="feedback err">{{ profiles.error }}</p>
    </main>
  </div>
</template>
