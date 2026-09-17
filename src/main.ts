import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import router from "./router";
import { bindTheme } from "./theme";
import "./styles/app.css";

bindTheme("system");

const app = createApp(App);
app.use(createPinia());
app.use(router);
app.mount("#app");
