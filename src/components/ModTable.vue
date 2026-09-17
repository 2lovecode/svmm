<script setup lang="ts">
import type { ModEntry } from "../types/mod";

defineProps<{
  mods: ModEntry[];
  loading: boolean;
  togglingPath: string | null;
}>();

const emit = defineEmits<{
  toggle: [mod: ModEntry, enabled: boolean];
}>();

function statusLabel(status: string): string {
  switch (status) {
    case "ok":
      return "正常";
    case "missing_manifest":
      return "清单异常";
    case "incompatible":
      return "不兼容";
    case "update_available":
      return "有更新";
    case "unofficial_update":
      return "非官方更新";
    case "broken":
      return "异常";
    default:
      return status;
  }
}
</script>

<template>
  <div class="mod-table-wrap">
    <table class="mod-table">
      <thead>
        <tr>
          <th class="col-toggle">启用</th>
          <th class="col-name">名称</th>
          <th class="col-author">作者</th>
          <th class="col-version">版本</th>
          <th class="col-status">状态</th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="loading && mods.length === 0">
          <td colspan="5" class="empty">正在加载模组列表…</td>
        </tr>
        <tr v-else-if="!loading && mods.length === 0">
          <td colspan="5" class="empty">暂无模组。请先在设置中配置路径后刷新。</td>
        </tr>
        <tr v-for="mod in mods" :key="mod.folderPath" :class="{ disabled: !mod.enabled }">
          <td class="col-toggle">
            <label class="switch">
              <input
                type="checkbox"
                :checked="mod.enabled"
                :disabled="togglingPath === mod.folderPath"
                @change="
                  emit(
                    'toggle',
                    mod,
                    ($event.target as HTMLInputElement).checked,
                  )
                "
              />
              <span class="switch-ui" />
              <span class="sr-only">{{ mod.enabled ? "禁用" : "启用" }} {{ mod.name }}</span>
            </label>
          </td>
          <td class="col-name">
            <div class="mod-name">{{ mod.name || "（未命名）" }}</div>
            <div v-if="mod.id" class="mod-id">{{ mod.id }}</div>
          </td>
          <td class="col-author">{{ mod.author || "—" }}</td>
          <td class="col-version">{{ mod.version || "—" }}</td>
          <td class="col-status">
            <span class="badge" :data-status="mod.status">{{ statusLabel(mod.status) }}</span>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
