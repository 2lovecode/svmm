import { createRouter, createWebHistory } from "vue-router";
import HomeView from "../views/HomeView.vue";
import BrowseView from "../views/BrowseView.vue";
import LibraryView from "../views/LibraryView.vue";
import ProfilesView from "../views/ProfilesView.vue";
import ProfileDetailView from "../views/ProfileDetailView.vue";
import SettingsView from "../views/SettingsView.vue";

const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", name: "home", component: HomeView },
    { path: "/browse", name: "browse", component: BrowseView },
    { path: "/library", name: "library", component: LibraryView },
    { path: "/profiles", name: "profiles", component: ProfilesView },
    { path: "/profiles/:id", name: "profile-detail", component: ProfileDetailView },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});

export default router;
