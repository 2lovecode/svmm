<script setup lang="ts">
defineProps<{
  step: 1 | 2 | 3;
}>();

const notes: Record<1 | 2 | 3, string> = {
  1: "这一步只是目录。入库后模组进本地库，不会进游戏。",
  2: "本地库是存货。更新换掉唯一的那一份；要玩，加进方案再应用。",
  3: "方案决定这次玩什么。应用之后，游戏 Mods 才会换成这一组。",
};
</script>

<template>
  <p class="hint layer-note">{{ notes[step] }}</p>
</template>
