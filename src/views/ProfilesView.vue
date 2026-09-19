<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import LayerGuide from "../components/LayerGuide.vue";
import { useProfilesStore } from "../stores/profiles";
import type { Profile } from "../types/mod";

const router = useRouter();
const profiles = useProfilesStore();
const newName = ref("");

function profileLabel(profile: Pick<Profile, "id" | "name">): string {
  if (profile.id === "default" || profile.name.toLowerCase() === "default") {
    return "默认方案";
  }
  return profile.name;
}

onMounted(() => {
  void profiles.load().catch(() => {
    /* error in store */
  });
});

async function onCreate() {
  const name = newName.value.trim();
  if (!name) return;
  try {
    const profile = await profiles.create(name);
    newName.value = "";
    await router.push({ name: "profile-detail", params: { id: profile.id } });
  } catch {
    /* store error */
  }
}

function openDetail(profile: Profile) {
  void router.push({ name: "profile-detail", params: { id: profile.id } });
}
</script>

<template>
  <div class="page-view">
    <header class="settings-header">
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
            placeholder="方案名称"
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
      </section>

      <section class="profiles-section">
        <h2>方案列表</h2>
        <p v-if="profiles.loading" class="hint">加载中…</p>
        <ul v-else-if="profiles.profiles.length === 0" class="profiles-list empty-list">
          <li>暂无方案</li>
        </ul>
        <ul v-else class="profiles-list">
          <li v-for="profile in profiles.profiles" :key="profile.id" class="profile-row">
            <button type="button" class="profile-link" @click="openDetail(profile)">
              <strong>{{ profileLabel(profile) }}</strong>
              <span class="profile-meta">{{ profile.modIds.length }} 个模组</span>
            </button>
          </li>
        </ul>
      </section>

      <p v-if="profiles.message" class="feedback ok">{{ profiles.message }}</p>
      <p v-if="profiles.error" class="feedback err">{{ profiles.error }}</p>
    </main>
  </div>
</template>
