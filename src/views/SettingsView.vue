<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../stores/settings";

const router = useRouter();
const store = useSettingsStore();

onMounted(async () => {
  try {
    await store.load();
  } catch {
    /* error in store */
  }
});

async function pickDirectory(field: "gamePath" | "modsPath") {
  const selected = await open({
    directory: true,
    multiple: false,
    title: field === "gamePath" ? "选择游戏目录" : "选择 Mods 目录",
  });
  if (typeof selected === "string") {
    store.patch({ [field]: selected });
  }
}

function onPathInput(
  field: "gamePath" | "smapiPath" | "modsPath",
  event: Event,
) {
  const value = (event.target as HTMLInputElement).value.trim();
  store.patch({ [field]: value.length > 0 ? value : null });
}

async function pickSmapiFile() {
  const selected = await open({
    directory: false,
    multiple: false,
    title: "选择 StardewModdingAPI.exe",
    filters: [{ name: "可执行文件", extensions: ["exe"] }],
  });
  if (typeof selected === "string") {
    store.patch({ smapiPath: selected });
  }
}

async function onDiscover() {
  try {
    await store.discover();
  } catch {
    /* store error */
  }
}

async function onSave() {
  try {
    await store.save();
  } catch {
    /* store error */
  }
}
</script>

<template>
  <div class="page settings-page">
    <header class="settings-header">
      <button type="button" class="btn btn-ghost" @click="router.push('/')">
        ← 返回
      </button>
      <h1>设置</h1>
    </header>

    <main class="settings-body">
      <p class="hint">配置星露谷物语、SMAPI 与 Mods 路径。留空时将尝试自动推导。</p>

      <div class="field">
        <label for="game-path">游戏目录</label>
        <div class="field-row">
          <input
            id="game-path"
            :value="store.settings.gamePath ?? ''"
            type="text"
            placeholder="例如 C:\Program Files (x86)\Steam\steamapps\common\Stardew Valley"
            @input="onPathInput('gamePath', $event)"
          />
          <button type="button" class="btn" @click="pickDirectory('gamePath')">浏览…</button>
        </div>
      </div>

      <div class="field">
        <label for="smapi-path">SMAPI 路径</label>
        <div class="field-row">
          <input
            id="smapi-path"
            :value="store.settings.smapiPath ?? ''"
            type="text"
            placeholder="StardewModdingAPI.exe 完整路径"
            @input="onPathInput('smapiPath', $event)"
          />
          <button type="button" class="btn" @click="pickSmapiFile">浏览…</button>
        </div>
      </div>

      <div class="field">
        <label for="mods-path">Mods 目录</label>
        <div class="field-row">
          <input
            id="mods-path"
            :value="store.settings.modsPath ?? ''"
            type="text"
            placeholder="默认：游戏目录下的 Mods"
            @input="onPathInput('modsPath', $event)"
          />
          <button type="button" class="btn" @click="pickDirectory('modsPath')">浏览…</button>
        </div>
      </div>

      <div class="settings-actions">
        <button type="button" class="btn" :disabled="store.loading" @click="onDiscover">
          自动探测
        </button>
        <button
          type="button"
          class="btn btn-primary"
          :disabled="store.saving || store.loading"
          @click="onSave"
        >
          {{ store.saving ? "保存中…" : "保存" }}
        </button>
      </div>

      <p v-if="store.message" class="feedback ok">{{ store.message }}</p>
      <p v-if="store.error" class="feedback err">{{ store.error }}</p>
    </main>
  </div>
</template>
