<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import * as api from "../api/tauri";
import { useModsStore } from "../stores/mods";
import { useProfilesStore } from "../stores/profiles";
import { useSettingsStore } from "../stores/settings";
import { formatAppError, type LibraryMod, type Profile, type ProfileState } from "../types/mod";

const route = useRoute();
const router = useRouter();
const profiles = useProfilesStore();
const mods = useModsStore();
const settings = useSettingsStore();

const detail = ref<ProfileState | null>(null);
const libraryMods = ref<LibraryMod[]>([]);
const selected = ref<string[]>([]);
const adding = ref(false);
const renaming = ref(false);
const renameDraft = ref("");
const loading = ref(false);

const profileId = computed(() => String(route.params.id ?? ""));

const availableMods = computed(() => {
  const owned = new Set(detail.value?.mods.map((mod) => mod.id) ?? []);
  return libraryMods.value.filter((mod) => !owned.has(mod.id));
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

function allAvailableSelected(): boolean {
  return (
    availableMods.value.length > 0 &&
    availableMods.value.every((mod) => selected.value.includes(mod.id))
  );
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
  const visible = new Set(availableMods.value.map((mod) => mod.id));
  if (checked) {
    const next = new Set(selected.value);
    for (const id of visible) next.add(id);
    selected.value = [...next];
  } else {
    selected.value = selected.value.filter((id) => !visible.has(id));
  }
}

async function load() {
  const id = profileId.value;
  if (!id) return;
  loading.value = true;
  profiles.error = null;
  try {
    detail.value = await api.profileDetail(id);
    libraryMods.value = await api.listLibrary();
    selected.value = selected.value.filter((modId) =>
      availableMods.value.some((mod) => mod.id === modId),
    );
    renameDraft.value = detail.value.profile.name;
  } catch (e) {
    profiles.error = formatAppError(e);
    detail.value = null;
  } finally {
    loading.value = false;
  }
}

async function onAddSelected() {
  if (!detail.value || selected.value.length === 0) return;
  adding.value = true;
  profiles.error = null;
  try {
    const count = selected.value.length;
    for (const id of selected.value) {
      await api.addProfileMod(detail.value.profile.id, id);
    }
    selected.value = [];
    await load();
    await profiles.load(profileId.value);
    profiles.message = `已添加 ${count} 个模组。要在游戏里用，请再应用该方案。`;
  } catch (e) {
    profiles.error = formatAppError(e);
  } finally {
    adding.value = false;
  }
}

async function onRemoveMod(modId: string) {
  if (!detail.value) return;
  try {
    await api.removeProfileMod(detail.value.profile.id, modId);
    await load();
    await profiles.load(profileId.value);
    profiles.message = "已从方案移出";
  } catch (e) {
    profiles.error = formatAppError(e);
  }
}

async function onApply() {
  if (!detail.value) return;
  try {
    await profiles.apply(detail.value.profile.id);
    await settings.load();
    await load();
    await mods.refresh();
  } catch {
    /* store */
  }
}

function startRename() {
  if (!detail.value) return;
  renaming.value = true;
  renameDraft.value = detail.value.profile.name;
}

async function confirmRename() {
  if (!detail.value) return;
  const name = renameDraft.value.trim();
  if (!name) return;
  try {
    await profiles.rename(detail.value.profile.id, name);
    renaming.value = false;
    await load();
  } catch {
    /* store */
  }
}

async function onDelete() {
  if (!detail.value || detail.value.profile.id === "default") return;
  const name = profileLabel(detail.value.profile);
  if (!window.confirm(`确定删除方案「${name}」？此操作不可撤销。`)) return;
  try {
    await profiles.remove(detail.value.profile.id);
    await router.push("/profiles");
  } catch {
    /* store */
  }
}

onMounted(() => {
  void load();
});

watch(profileId, () => {
  selected.value = [];
  renaming.value = false;
  void load();
});
</script>

<template>
  <div class="page-view">
    <header class="settings-header">
      <button type="button" class="btn btn-ghost" @click="router.push('/profiles')">← 方案</button>
      <h1>{{ detail ? profileLabel(detail.profile) : "方案" }}</h1>
    </header>
    <main class="settings-body profiles-body">
      <p v-if="loading" class="hint">加载中…</p>
      <template v-else-if="detail">
        <section class="profiles-section">
          <p class="hint">{{ detail.applied ? "已应用到游戏目录" : "尚未应用到游戏目录" }}</p>
          <div v-if="renaming" class="field-row">
            <input
              v-model="renameDraft"
              type="text"
              class="rename-input"
              :disabled="profiles.mutating"
              @keydown.enter="confirmRename"
              @keydown.escape="renaming = false"
            />
            <button
              type="button"
              class="btn btn-primary"
              :disabled="profiles.mutating || !renameDraft.trim()"
              @click="confirmRename"
            >
              保存
            </button>
            <button type="button" class="btn btn-ghost" @click="renaming = false">取消</button>
          </div>
          <div v-else class="profile-actions">
            <button
              type="button"
              class="btn btn-primary"
              :disabled="profiles.applying"
              @click="onApply"
            >
              {{ profiles.applying ? "应用中…" : "应用" }}
            </button>
            <button type="button" class="btn" :disabled="profiles.mutating" @click="startRename">
              重命名
            </button>
            <button
              type="button"
              class="btn btn-danger"
              :disabled="profiles.mutating || detail.profile.id === 'default'"
              :title="detail.profile.id === 'default' ? '不能删除默认方案' : undefined"
              @click="onDelete"
            >
              删除
            </button>
          </div>
        </section>

        <section class="profiles-section">
          <h2>方案中的模组</h2>
          <ul class="profiles-list">
            <li v-if="detail.mods.length === 0">还没有模组。</li>
            <li v-for="mod in detail.mods" :key="mod.id" class="profile-row">
              <div class="profile-main">
                <strong>{{ mod.name }}</strong>
                <span class="profile-meta">当前 {{ mod.version || "未知" }} · {{ mod.id }}</span>
              </div>
              <button type="button" class="btn" @click="onRemoveMod(mod.id)">移出</button>
            </li>
          </ul>
        </section>

        <section class="profiles-section">
          <h2>从本地库添加</h2>
          <div class="field-row">
            <button
              type="button"
              class="btn btn-primary"
              :disabled="selected.length === 0 || adding"
              @click="onAddSelected"
            >
              {{ adding ? "添加中…" : `添加${selected.length ? `（${selected.length}）` : ""}` }}
            </button>
          </div>
          <ul v-if="availableMods.length === 0" class="profiles-list empty-list">
            <li>本地库里没有可添加的模组。</li>
          </ul>
          <ul v-else class="profiles-list">
            <li class="profile-row select-all-row">
              <label class="mod-check">
                <input type="checkbox" :checked="allAvailableSelected()" @change="onToggleAll" />
                <span>全选</span>
              </label>
              <span v-if="selected.length" class="profile-meta">已选 {{ selected.length }}</span>
            </li>
            <li v-for="mod in availableMods" :key="mod.id" class="profile-row">
              <label class="mod-check">
                <input
                  type="checkbox"
                  :checked="isSelected(mod.id)"
                  @change="onToggle(mod.id, $event)"
                />
                <span class="sr-only">选择 {{ mod.name }}</span>
              </label>
              <div class="profile-main">
                <strong>{{ mod.name }}</strong>
                <span class="profile-meta">当前 {{ mod.version || "未知" }} · {{ mod.id }}</span>
              </div>
            </li>
          </ul>
        </section>
      </template>
      <p v-else-if="!loading" class="hint">找不到这个方案。</p>
      <p v-if="profiles.message" class="feedback ok">{{ profiles.message }}</p>
      <p v-if="profiles.error" class="feedback err">{{ profiles.error }}</p>
    </main>
  </div>
</template>
