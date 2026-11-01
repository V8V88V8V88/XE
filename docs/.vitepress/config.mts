import { readFileSync } from "node:fs";
import { defineConfig } from "vitepress";
import { xeDark, xeLight } from "./code-theme";

const siteUrl = "https://xe-lang.vercel.app";
const repoUrl = "https://github.com/V8V88V8V88/XE";
const description =
  "A small, indentation-based language that compiles to Rust and builds standalone native executables.";

// Highlight ```xe blocks with the VS Code extension's grammar (one source of truth).
// Falls back to Python highlighting if the grammar file is not available at build time.
function xeLanguage() {
  try {
    const url = new URL("../../editors/vscode/syntaxes/xe.tmLanguage.json", import.meta.url);
    const grammar = JSON.parse(readFileSync(url, "utf8"));
    return { languages: [{ ...grammar, name: "xe", aliases: ["XE"] }] };
  } catch {
    return { languageAlias: { xe: "python" } };
  }
}

// The site shows the version from Cargo.toml, which scripts/release.sh bumps on every
// release, so the docs never need a manual version edit.
function xeVersion(): string | null {
  try {
    const cargo = readFileSync(new URL("../../Cargo.toml", import.meta.url), "utf8");
    return cargo.match(/^version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
  } catch {
    return null;
  }
}

const version = xeVersion();

export default defineConfig({
  title: "XE",
  description,
  lastUpdated: true,
  sitemap: { hostname: siteUrl },
  head: [
    ["link", { rel: "icon", type: "image/png", href: "/xe-icon.png" }],
    ["link", { rel: "apple-touch-icon", href: "/xe-icon.png" }],
    ["meta", { name: "theme-color", content: "#11100d" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: "XE" }],
    ["meta", { property: "og:title", content: "XE programming language" }],
    ["meta", { property: "og:description", content: description }],
    ["meta", { property: "og:image", content: `${siteUrl}/xe-icon.png` }],
    ["meta", { name: "twitter:card", content: "summary" }],
  ],
  markdown: {
    // Warm palette that matches the site's brand colours (see code-theme.ts)
    theme: { light: xeLight, dark: xeDark },
    ...xeLanguage(),
  },
  themeConfig: {
    logo: { light: "/xe-mark-light.png", dark: "/xe-mark-dark.png", alt: "XE" },
    siteTitle: false,
    nav: [
      { text: "Docs", link: "/guide/getting-started", activeMatch: "^/guide/(?!examples)" },
      { text: "Reference", link: "/reference/language", activeMatch: "/reference/" },
      { text: "Examples", link: "/guide/examples" },
      { text: "Status", link: "/reference/status" },
      ...(version
        ? [
            {
              text: `v${version}`,
              items: [
                { text: "What's new", link: "/reference/status" },
                { text: "All releases", link: `${repoUrl}/releases` },
              ],
            },
          ]
        : []),
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Introduction",
          collapsed: false,
          items: [
            { text: "Getting Started", link: "/guide/getting-started" },
            { text: "Editor Support", link: "/guide/editor-support" },
          ],
        },
        {
          text: "Language",
          collapsed: false,
          items: [
            { text: "Language Basics", link: "/guide/language-basics" },
            { text: "Syntax and Blocks", link: "/guide/syntax-and-blocks" },
            { text: "Types and Values", link: "/guide/types-and-values" },
            { text: "Control Flow", link: "/guide/control-flow" },
            { text: "Functions and Scope", link: "/guide/functions-and-scope" },
            { text: "Runtime Behavior and Errors", link: "/guide/runtime-and-errors" },
          ],
        },
        {
          text: "More",
          collapsed: false,
          items: [
            { text: "Examples", link: "/guide/examples" },
            { text: "FAQ", link: "/guide/faq" },
            { text: "Contributing", link: "/guide/contributing" },
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
    socialLinks: [{ icon: "github", link: repoUrl }],
    editLink: {
      pattern: `${repoUrl}/edit/main/docs/:path`,
      text: "Edit this page on GitHub",
    },
    lastUpdated: {
      text: "Last updated",
      formatOptions: { dateStyle: "medium" },
    },
    notFound: {
      title: "Page not found",
      quote: "This page does not exist, or it moved when the docs were reorganised.",
      linkLabel: "Go to the home page",
      linkText: "Back to home",
    },
    footer: {
      message:
        'Pre-alpha language project built for learning and experimentation.<br>' +
        `<a href="${repoUrl}">GitHub</a> · ` +
        `<a href="${repoUrl}/releases">Releases</a> · ` +
        '<a href="https://marketplace.visualstudio.com/items?itemName=xe-lang.xe-language">VS Code extension</a> · ' +
        '<a href="/guide/examples">Examples</a> · ' +
        '<a href="/guide/faq">FAQ</a> · ' +
        '<a href="/guide/contributing">Contributing</a>',
      copyright: "Released under GPL-3.0-or-later",
    },
    outline: "deep",
    search: {
      provider: "local",
    },
  },
});
