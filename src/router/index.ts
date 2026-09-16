import { createRouter, createWebHistory } from "vue-router";
import HomeView from "../views/HomeView.vue";
import ProfilesView from "../views/ProfilesView.vue";
import SettingsView from "../views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "home", component: HomeView },
    { path: "/profiles", name: "profiles", component: ProfilesView },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});

export default router;
