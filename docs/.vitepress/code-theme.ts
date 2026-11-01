// Code highlighting themes in the site's warm palette.
// The colours match the home page code window (see --xe-code-* in theme/custom.css).

type Palette = {
  fg: string;
  keyword: string;
  fn: string;
  string: string;
  number: string;
  type: string;
  comment: string;
  operator: string;
};

function makeTheme(name: string, type: "light" | "dark", c: Palette) {
  return {
    name,
    type,
    colors: { "editor.foreground": c.fg },
    tokenColors: [
      { scope: ["comment", "punctuation.definition.comment"], settings: { foreground: c.comment, fontStyle: "italic" } },
      {
        scope: ["keyword", "storage", "storage.type", "storage.modifier", "keyword.control"],
        settings: { foreground: c.keyword },
      },
      {
        scope: ["keyword.operator", "punctuation.separator", "punctuation.accessor", "punctuation"],
        settings: { foreground: c.operator },
      },
      {
        scope: ["string", "punctuation.definition.string", "constant.character.escape"],
        settings: { foreground: c.string },
      },
      {
        scope: ["constant.numeric", "constant.language", "constant.other", "variable.other.constant"],
        settings: { foreground: c.number },
      },
      {
        scope: ["entity.name.function", "support.function", "meta.function-call.generic", "variable.function"],
        settings: { foreground: c.fn },
      },
      {
        scope: ["entity.name.type", "support.type", "entity.name.class", "storage.type.rust", "entity.name.namespace"],
        settings: { foreground: c.type },
      },
      { scope: ["variable.parameter", "variable.other", "variable"], settings: { foreground: c.fg } },
      { scope: ["markup.heading"], settings: { foreground: c.keyword, fontStyle: "bold" } },
      { scope: ["markup.inserted"], settings: { foreground: c.string } },
      { scope: ["markup.deleted"], settings: { foreground: c.keyword } },
    ],
  };
}

export const xeDark = makeTheme("xe-dark", "dark", {
  fg: "#e9e1d3",
  keyword: "#e0965a",
  fn: "#e9c99b",
  string: "#a8c48a",
  number: "#d7a6e0",
  type: "#8fbfb0",
  comment: "#857b6c",
  operator: "#b8ad9b",
});

export const xeLight = makeTheme("xe-light", "light", {
  fg: "#2a241c",
  keyword: "#b0541c",
  fn: "#7d5512",
  string: "#4a7a28",
  number: "#8a3f9c",
  type: "#2f7468",
  comment: "#8b8173",
  operator: "#5e564b",
});
