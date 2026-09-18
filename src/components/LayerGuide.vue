<script setup lang="ts">
defineProps<{
  step: 1 | 2 | 3;
}>();

const notes: Record<1 | 2 | 3, string> = {
  1: "这一步只是目录。入库后模组进本地库，不会进游戏。",
  2: "本地库是存货。更新和降级换掉唯一的那一份；要玩，加进方案再应用。",
  3: "方案决定这次玩什么。应用之后，游戏 Mods 才会换成这一组。",
};
</script>

<template>
  <ol class="layer-guide">
    <li>
      <router-link class="btn" :class="step === 1 ? 'btn-primary' : 'btn-ghost'" to="/browse">
        1. 浏览入库
      </router-link>
    </li>
    <li>
      <router-link class="btn" :class="step === 2 ? 'btn-primary' : 'btn-ghost'" to="/library">
        2. 本地库
      </router-link>
    </li>
    <li>
      <router-link class="btn" :class="step === 3 ? 'btn-primary' : 'btn-ghost'" to="/">
        3. 方案并应用
      </router-link>
    </li>
    <li class="hint">{{ notes[step] }}</li>
  </ol>
</template>
