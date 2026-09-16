<script setup lang="ts">
defineProps<{
  message: string;
  enabledCount: number;
  totalCount: number;
  error: string | null;
}>();
</script>

<template>
  <footer class="statusbar">
    <div class="status-left">
      <span>{{ message }}</span>
      <span v-if="error" class="status-error" :title="error">{{ error }}</span>
    </div>
    <div class="status-right">
      已启用 {{ enabledCount }} / {{ totalCount }}
    </div>
  </footer>
</template>
