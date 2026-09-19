<script setup lang="ts">
import { RouterView } from "vue-router";
import TopBar from "./components/TopBar.vue";
</script>

<template>
  <div class="app-shell">
    <div class="page">
      <TopBar />
      <RouterView />
    </div>
  </div>
</template>
