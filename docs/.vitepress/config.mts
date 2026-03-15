import { defineConfig } from "vitepress";

export default defineConfig({
  title: "XE",
  description: "Documentation for the XE programming language",
  lang: "en-US",
  cleanUrls: true,
  lastUpdated: true,
  markdown: {
    languageAlias: {
      xe: "python"
    }
  },
  themeConfig: {
    logo: "/XElogo.png",
    siteTitle: "XE Docs",
    search: {
      provider: "local"
    },
    nav: [
      { text: "Guide", link: "/guide/getting-started" },
      { text: "Examples", link: "/guide/examples" },
      { text: "Reference", link: "/reference/cli" },
      { text: "GitHub", link: "https://github.com/V8V88V8V88/XE" }
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Guide",
          items: [
            { text: "Getting Started", link: "/guide/getting-started" },
            { text: "Language Basics", link: "/guide/language-basics" },
            { text: "Examples", link: "/guide/examples" }
          ]
        }
      ],
      "/reference/": [
        {
          text: "Reference",
          items: [
            { text: "CLI", link: "/reference/cli" },
            { text: "Language", link: "/reference/language" },
            { text: "Project Status", link: "/reference/status" }
          ]
        }
      ]
    },
    socialLinks: [
      { icon: "github", link: "https://github.com/V8V88V8V88/XE" }
    ],
    footer: {
      message: "Pre-alpha programming language and compiler project.",
      copyright: "MIT License"
    }
  }
});
