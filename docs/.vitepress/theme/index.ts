import DefaultTheme from "vitepress/theme";
import "./custom.css";
import XeHome from "./components/XeHome.vue";

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("XeHome", XeHome);
    
    if (typeof window !== "undefined") {
      const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            entry.target.classList.add("xe-visible");
          }
        });
      }, { threshold: 0.1 });

      window.addEventListener("load", () => {
        document.querySelectorAll(".xe-reveal").forEach(el => observer.observe(el));
      });
      
      // VitePress dynamic routing support
      app.mixin({
        mounted() {
          this.$nextTick(() => {
            document.querySelectorAll(".xe-reveal").forEach(el => observer.observe(el));
          });
        }
      });
    }
  },
};
