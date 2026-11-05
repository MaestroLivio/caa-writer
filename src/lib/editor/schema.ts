// Schema ProseMirror dell'editor CAA.
//
// Ogni paragrafo porta un `pid` stabile: serve a collegare i pittogrammi
// alle unità comunicative senza usare indici fragili (che cambiano a ogni
// modifica del testo). La formattazione è separata dal contenuto.

import { Schema } from "prosemirror-model";
import { tableNodes } from "prosemirror-tables";

const table = tableNodes({
  tableGroup: "block",
  cellContent: "block+",
  cellAttributes: {},
});

export const schema = new Schema({
  nodes: {
    doc: { content: "block+" },

    paragraph: {
      content: "inline*",
      group: "block",
      attrs: { pid: { default: null }, align: { default: null } },
      parseDOM: [
        {
          tag: "p",
          getAttrs: (dom: HTMLElement) => ({
            pid: dom.getAttribute("data-pid") || null,
            align: dom.style.textAlign || null,
          }),
        },
      ],
      toDOM: (node) => [
        "p",
        {
          "data-pid": node.attrs.pid ?? "",
          style: node.attrs.align ? `text-align:${node.attrs.align}` : "",
        },
        0,
      ],
    },

    heading: {
      content: "inline*",
      group: "block",
      defining: true,
      attrs: { level: { default: 1 }, pid: { default: null }, align: { default: null } },
      parseDOM: [
        {
          tag: "h1",
          getAttrs: (dom: HTMLElement) => ({
            level: 1,
            pid: dom.getAttribute("data-pid") || null,
            align: dom.style.textAlign || null,
          }),
        },
        {
          tag: "h2",
          getAttrs: (dom: HTMLElement) => ({
            level: 2,
            pid: dom.getAttribute("data-pid") || null,
            align: dom.style.textAlign || null,
          }),
        },
        {
          tag: "h3",
          getAttrs: (dom: HTMLElement) => ({
            level: 3,
            pid: dom.getAttribute("data-pid") || null,
            align: dom.style.textAlign || null,
          }),
        },
      ],
      toDOM: (node) => [
        `h${node.attrs.level}`,
        {
          "data-pid": node.attrs.pid ?? "",
          style: node.attrs.align ? `text-align:${node.attrs.align}` : "",
        },
        0,
      ],
    },

    text: { group: "inline" },

    hard_break: {
      inline: true,
      group: "inline",
      selectable: false,
      parseDOM: [{ tag: "br" }],
      toDOM: () => ["br"],
    },

    bullet_list: {
      content: "list_item+",
      group: "block",
      parseDOM: [{ tag: "ul" }],
      toDOM: () => ["ul", 0],
    },

    ordered_list: {
      content: "list_item+",
      group: "block",
      attrs: { order: { default: 1 } },
      parseDOM: [{ tag: "ol" }],
      toDOM: () => ["ol", 0],
    },

    list_item: {
      content: "paragraph block*",
      defining: true,
      parseDOM: [{ tag: "li" }],
      toDOM: () => ["li", 0],
    },

    ...table,
  },

  marks: {
    strong: {
      parseDOM: [{ tag: "strong" }, { tag: "b" }],
      toDOM: () => ["strong", 0],
    },
    em: {
      parseDOM: [{ tag: "em" }, { tag: "i" }],
      toDOM: () => ["em", 0],
    },
    underline: {
      parseDOM: [{ tag: "u" }],
      toDOM: () => ["u", 0],
    },
    text_style: {
      attrs: { color: { default: null }, font_size: { default: null } },
      parseDOM: [
        {
          tag: "span",
          getAttrs: (dom: HTMLElement) => {
            const style = dom.style;
            if (!style.color && !style.fontSize) return false;
            return {
              color: style.color || null,
              font_size: style.fontSize || null,
            };
          },
        },
      ],
      toDOM: (mark) => {
        const styles: string[] = [];
        if (mark.attrs.color) styles.push(`color:${mark.attrs.color}`);
        if (mark.attrs.font_size) styles.push(`font-size:${mark.attrs.font_size}`);
        return ["span", { style: styles.join(";") }, 0];
      },
    },
  },
});

export type NodeName = keyof typeof schema.nodes;
