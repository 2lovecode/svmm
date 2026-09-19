<script setup lang="ts">
import { useRoute } from "vue-router";

const route = useRoute();
</script>

<template>
  <header class="topbar">
    <router-link class="brand" to="/" title="首页" aria-label="首页">
      <svg class="junimo" viewBox="0 0 8 8" aria-hidden="true">
        <g shape-rendering="crispEdges">
          <rect x="2" y="0" width="4" height="1" fill="#4c6840" />
          <rect x="1" y="1" width="6" height="1" fill="#4c6840" />
          <rect x="1" y="2" width="6" height="1" fill="#4c6840" />
          <rect x="1" y="3" width="6" height="1" fill="#4c6840" />
          <rect x="1" y="4" width="6" height="1" fill="#5c7850" />
          <rect x="1" y="5" width="6" height="1" fill="#4c6840" />
          <rect x="2" y="6" width="1" height="1" fill="#4c6840" />
          <rect x="5" y="6" width="1" height="1" fill="#4c6840" />
          <rect x="2" y="2" width="1" height="1" fill="#f7f1df" />
          <rect x="5" y="2" width="1" height="1" fill="#f7f1df" />
          <rect x="2" y="3" width="1" height="1" fill="#1c140e" />
          <rect x="5" y="3" width="1" height="1" fill="#1c140e" />
        </g>
      </svg>
      <div class="brand-text">
        <strong>SVMM</strong>
        <span class="brand-sub">星露谷模组管理</span>
      </div>
    </router-link>
    <div class="topbar-end">
      <nav class="actions">
        <router-link
          class="btn btn-ghost"
          :class="{ 'is-current': route.path.startsWith('/browse') }"
          to="/browse"
        >
          浏览
        </router-link>
        <router-link
          class="btn btn-ghost"
          :class="{ 'is-current': route.path.startsWith('/library') }"
          to="/library"
        >
          本地库
        </router-link>
        <router-link
          class="btn btn-ghost"
          :class="{ 'is-current': route.path.startsWith('/profiles') }"
          to="/profiles"
        >
          方案
        </router-link>
      </nav>
      <router-link
        class="btn btn-ghost btn-icon"
        :class="{ 'is-current': route.path.startsWith('/settings') }"
        to="/settings"
        aria-label="设置"
        title="设置"
      >
        <svg class="icon-gear" viewBox="0 0 24 24" aria-hidden="true">
          <path
            fill="currentColor"
            d="M19.4 13.5a7.7 7.7 0 0 0 .1-1.5 7.7 7.7 0 0 0-.1-1.5l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-2.6-1.5L13.8 2h-3.6l-.6 2.9a7.4 7.4 0 0 0-2.6 1.5l-2.5-1-2 3.5 2.1 1.6a7.7 7.7 0 0 0-.1 1.5 7.7 7.7 0 0 0 .1 1.5l-2.1 1.6 2 3.5 2.5-1a7.4 7.4 0 0 0 2.6 1.5l.6 2.9h3.6l.6-2.9a7.4 7.4 0 0 0 2.6-1.5l2.5 1 2-3.5-2.1-1.6ZM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7Z"
          />
        </svg>
      </router-link>
    </div>
  </header>
</template>
