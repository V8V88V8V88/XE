import DefaultTheme from "vitepress/theme";
import "./custom.css";
import XeHome from "./components/XeHome.vue";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("XeHome", XeHome);
  },
};
