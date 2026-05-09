import { defineConfig } from "vitepress";

export default defineConfig({
  title: "XE",
  description: "A small programming language that compiles into Rust and runs as a native binary.",
  head: [["link", { rel: "icon", type: "image/png", href: "/XElogo.png" }]],
  markdown: {
    languageAlias: {
      xe: "python",
    },
  },
  themeConfig: {
    logo: "/XElogo.png",
    nav: [
      { text: "Docs", link: "/guide/getting-started" },
      { text: "Reference", link: "/reference/language" },
      { text: "Status", link: "/reference/status" },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Docs",
          items: [
            { text: "Getting Started", link: "/guide/getting-started" },
            { text: "Language Basics", link: "/guide/language-basics" },
            { text: "Syntax and Blocks", link: "/guide/syntax-and-blocks" },
            { text: "Types and Values", link: "/guide/types-and-values" },
            { text: "Control Flow", link: "/guide/control-flow" },
            { text: "Functions and Scope", link: "/guide/functions-and-scope" },
            { text: "Runtime Behavior and Errors", link: "/guide/runtime-and-errors" },
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
            { text: "Keywords", link: "/guide/keywords" },
            { text: "Status", link: "/reference/status" },
          ],
        },
      ],
    },
    socialLinks: [{ icon: "github", link: "https://github.com/V8V88V8V88/XE" }],
    footer: {
      message: "Pre-alpha language project built for learning and experimentation.",
      copyright: "Released under GPL-3.0-or-later",
    },
    outline: "deep",
    search: {
      provider: "local",
    },
  },
});
