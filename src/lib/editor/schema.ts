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

/** Attributi di paragrafo/titolo comuni, serializzati nello stile DOM. */
function paragraphStyle(attrs: Record<string, unknown>): string {
  const styles: string[] = [];
  if (attrs.align) styles.push(`text-align:${attrs.align}`);
  const indent = Number(attrs.indent ?? 0);
  if (indent > 0) styles.push(`padding-left:${indent * 24}px`);
  if (attrs.line_height) styles.push(`line-height:${attrs.line_height}`);
  if (attrs.space_before) styles.push(`margin-top:${attrs.space_before}`);
  if (attrs.space_after) styles.push(`margin-bottom:${attrs.space_after}`);
  if (attrs.border) styles.push("border:1px solid var(--border)", "border-radius:6px", "padding:6px 10px");
  if (attrs.page_break) styles.push("break-before:page");
  return styles.join(";");
}

export const schema = new Schema({
  nodes: {
    doc: { content: "block+" },

    paragraph: {
      content: "inline*",
      group: "block",
      attrs: {
        pid: { default: null },
        align: { default: null },
        indent: { default: 0 },
        line_height: { default: null },
        space_before: { default: null },
        space_after: { default: null },
        border: { default: false },
        page_break: { default: false },
      },
      parseDOM: [
        {
          tag: "p",
          getAttrs: (dom: HTMLElement) => ({
            pid: dom.getAttribute("data-pid") || null,
            align: dom.style.textAlign || null,
            indent: Number(dom.getAttribute("data-indent") || 0),
            line_height: dom.style.lineHeight || null,
            space_before: dom.style.marginTop || null,
            space_after: dom.style.marginBottom || null,
            border: dom.getAttribute("data-border") === "true",
            page_break: dom.getAttribute("data-page-break") === "true",
          }),
        },
      ],
      toDOM: (node) => [
        "p",
        {
          "data-pid": node.attrs.pid ?? "",
          "data-indent": String(node.attrs.indent ?? 0),
          "data-border": node.attrs.border ? "true" : "false",
          "data-page-break": node.attrs.page_break ? "true" : "false",
          class: node.attrs.page_break ? "page-break" : undefined,
          style: paragraphStyle(node.attrs),
        },
        0,
      ],
    },

    heading: {
      content: "inline*",
      group: "block",
      defining: true,
      attrs: {
        level: { default: 1 },
        pid: { default: null },
        align: { default: null },
        indent: { default: 0 },
        line_height: { default: null },
        space_before: { default: null },
        space_after: { default: null },
        border: { default: false },
        page_break: { default: false },
      },
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
          class: node.attrs.page_break ? "page-break" : undefined,
          style: paragraphStyle(node.attrs),
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
    strike: {
      parseDOM: [{ tag: "s" }, { tag: "strike" }, { tag: "del" }],
      toDOM: () => ["s", 0],
    },
    highlight: {
      attrs: { color: { default: "#fff59d" } },
      parseDOM: [
        {
          tag: "mark",
          getAttrs: (dom: HTMLElement) => ({
            color: dom.style.backgroundColor || "#fff59d",
          }),
        },
      ],
      toDOM: (mark) => ["mark", { style: `background-color:${mark.attrs.color}` }, 0],
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
