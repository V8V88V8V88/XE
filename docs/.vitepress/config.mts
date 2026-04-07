import { defineConfig } from "vitepress";

export default defineConfig({
  title: "XE",
  description: "A small programming language that compiles into Rust and runs as a native binary.",
  themeConfig: {
    logo: "/XElogo.png",
    nav: [
      { text: "Guide", link: "/guide/getting-started" },
      { text: "Reference", link: "/reference/language" },
      { text: "Status", link: "/reference/status" },
      { text: "GitHub", link: "https://github.com/V8V88V8V88/XE" },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Guide",
          items: [
            { text: "Getting Started", link: "/guide/getting-started" },
            { text: "Language Basics", link: "/guide/language-basics" },
            { text: "Examples", link: "/guide/examples" },
          ],
        },
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "CLI", link: "/reference/cli" },
            { text: "Language", link: "/reference/language" },
            { text: "Status", link: "/reference/status" },
          ],
        },
      ],
    },
    socialLinks: [{ icon: "github", link: "https://github.com/V8V88V8V88/XE" }],
    footer: {
      message: "Pre-alpha language project built for learning and experimentation.",
      copyright: "Released under the MIT License",
    },
    outline: "deep",
    search: {
      provider: "local",
    },
  },
});
