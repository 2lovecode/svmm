<script setup lang="ts">
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../stores/settings";

const router = useRouter();
const store = useSettingsStore();

const isMac =
  navigator.userAgent.includes("Macintosh") || navigator.platform.startsWith("Mac");
const smapiFileName = isMac ? "StardewModdingAPI" : "StardewModdingAPI.exe";
const gamePathPlaceholder = isMac
  ? "~/Library/Application Support/Steam/steamapps/common/Stardew Valley/Contents/MacOS"
  : "例如 C:\\Program Files (x86)\\Steam\\steamapps\\common\\Stardew Valley";

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

function onProxyInput(event: Event) {
  const value = (event.target as HTMLInputElement).value.trim();
  store.patch({ downloadProxy: value.length > 0 ? value : null });
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
    title: `选择 ${smapiFileName}`,
    ...(isMac
      ? {}
      : { filters: [{ name: "可执行文件", extensions: ["exe"] }] }),
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

async function onSaveNexusKey() {
  try {
    await store.saveNexusKey();
  } catch {
    /* store error */
  }
}

async function onValidateNexus() {
  try {
    await store.validateNexusKey();
  } catch {
    /* store error */
  }
}

async function onClearNexusKey() {
  try {
    await store.clearNexusKey();
  } catch {
    /* store error */
  }
}

function onThemeChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  store.patch({ theme: value });
}

function onCheckUpdatesToggle(event: Event) {
  const checked = (event.target as HTMLInputElement).checked;
  store.patch({ checkUpdatesOnStartup: checked });
}

async function onOpenLogDir() {
  try {
    await store.openLogDir();
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
      <section class="settings-card">
        <h2>游戏</h2>
        <p class="hint">
          配置星露谷物语、SMAPI 与 Mods 路径。留空时将尝试自动推导。
          <template v-if="isMac">
            macOS 上请指向 Contents/MacOS，或直接选择游戏文件夹 / .app。
          </template>
        </p>

        <div class="field">
          <label for="game-path">游戏目录</label>
          <div class="field-row">
            <input
              id="game-path"
              :value="store.settings.gamePath ?? ''"
              type="text"
              :placeholder="gamePathPlaceholder"
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
              :placeholder="`${smapiFileName} 完整路径`"
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
        </div>
      </section>

      <section class="settings-card">
        <h2>下载</h2>
        <p class="hint">用于下载 SMAPI 和 Nexus 模组。</p>

        <div class="field">
          <label for="download-proxy">代理地址</label>
          <input
            id="download-proxy"
            class="text-input mono"
            :value="store.settings.downloadProxy ?? ''"
            type="text"
            spellcheck="false"
            autocomplete="off"
            placeholder="留空则使用系统代理"
            @input="onProxyInput"
          />
          <p class="field-note">
            例如 <code>http://127.0.0.1:7890</code> 或
            <code>socks5://127.0.0.1:7890</code>。只写
            <code>127.0.0.1:7890</code> 时按 HTTP 代理处理。
          </p>
        </div>
      </section>

      <section class="settings-card">
        <h2>外观与行为</h2>

        <div class="field">
          <label for="theme">主题</label>
          <select
            id="theme"
            :value="store.settings.theme"
            @change="onThemeChange"
          >
            <option value="system">跟随系统</option>
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </div>

        <div class="field">
          <div class="toggle-row">
            <span class="toggle-label">启动时检查模组更新</span>
            <label class="switch">
              <input
                type="checkbox"
                :checked="store.settings.checkUpdatesOnStartup"
                @change="onCheckUpdatesToggle"
              />
              <span class="switch-ui" />
              <span class="sr-only">启动时检查模组更新</span>
            </label>
          </div>
        </div>

        <div class="settings-actions">
          <button
            type="button"
            class="btn"
            :disabled="store.openingLogDir"
            @click="onOpenLogDir"
          >
            {{ store.openingLogDir ? "打开中…" : "打开日志目录" }}
          </button>
        </div>
      </section>

      <div class="settings-savebar">
        <p class="field-note">路径、代理和外观修改后需要保存。</p>
        <button
          type="button"
          class="btn btn-primary"
          :disabled="store.saving || store.loading"
          @click="onSave"
        >
          {{ store.saving ? "保存中…" : "保存" }}
        </button>
      </div>

      <section class="settings-card">
        <h2>Nexus Mods</h2>
        <p class="hint">
          API 密钥不会写入设置文件，只保存在本机应用数据目录中。
          状态：
          <span
            :class="
              store.nexusHasKey || store.nexusUser
                ? 'nexus-status ok'
                : 'nexus-status muted'
            "
          >
            {{ store.nexusStatusLabel }}
          </span>
        </p>

        <div class="field">
          <label for="nexus-key">API 密钥</label>
          <div class="field-row">
            <input
              id="nexus-key"
              v-model="store.nexusKeyInput"
              class="mono"
              type="password"
              autocomplete="off"
              placeholder="粘贴 Nexus API Key"
            />
          </div>
        </div>

        <div class="settings-actions">
          <button
            type="button"
            class="btn btn-primary"
            :disabled="store.nexusBusy || !store.nexusKeyInput.trim()"
            @click="onSaveNexusKey"
          >
            保存密钥
          </button>
          <button
            type="button"
            class="btn"
            :disabled="store.nexusBusy || !store.nexusHasKey"
            @click="onValidateNexus"
          >
            验证
          </button>
          <button
            type="button"
            class="btn btn-danger"
            :disabled="store.nexusBusy || !store.nexusHasKey"
            @click="onClearNexusKey"
          >
            清除
          </button>
        </div>
      </section>

      <p v-if="store.message" class="feedback ok">{{ store.message }}</p>
      <p v-if="store.error" class="feedback err">{{ store.error }}</p>
    </main>
  </div>
</template>
