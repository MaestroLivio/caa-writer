<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { ComponentProps } from "svelte";
  import { api, isTauri } from "../api";
  import CaaEditor from "../editor/CaaEditor.svelte";
  import ElementTextEditor from "./ElementTextEditor.svelte";
  import { mergeTextElements } from "../universal/text-sync";
  import { buildPhraseGrid } from "../universal/phrase";
  import {
    elementSegments,
    storedCaaText,
    supportsCaaText,
    unitInlineStyle,
    unitKey,
    unitAccessibleTitle,
  } from "../universal/element-caa";
  import type { StoredCaaToken } from "../universal/element-caa";
  import type { CaaToken } from "../types";
  import type {
    Alternative,
    OnlineResult,
    Profile,
    RenderedDocument,
    RenderedElement,
    RenderedPage,
    UniversalCommand,
    UniversalDocument,
    UniversalElement,
    UniversalElementSelection,
    UniversalPage,
    UniversalStyle,
  } from "../types";

  interface Props {
    document: UniversalDocument;
    path?: string | null;
    readOnly?: boolean;
    profile?: Profile | null;
    onSave: (document: UniversalDocument) => Promise<void>;
    onSavePlayer?: (document: UniversalDocument) => Promise<void>;
    onClose: () => void;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
    onSpeak: (text: string) => void;
    onExport?: (document: UniversalDocument, format: "html" | "svg" | "pdf" | "png" | "docx" | "pptx") => Promise<void>;
    canEdit?: boolean;
    onOpen?: () => void;
    onImport?: () => void;
    onEdit?: () => void;
    onToggleFullscreen?: () => void;
    onOpenAi?: () => void;
    onOpenStudio?: () => void;
    /// Proprietà del motore CAA per le pagine di testo incorporate.
    caa?: ComponentProps<typeof CaaEditor>;
    onEditorReady?: (editor: CaaEditor | undefined) => void;
    onDirtyChange?: (dirty: boolean) => void;
    /// Vero quando l'editor è dentro la shell dell'app: il Ribbon sostituisce
    /// la barra interna, evitando Home, titolo e comandi duplicati.
    embedded?: boolean;
    onTitleChange?: (title: string) => void;
    onDocumentChange?: (document: UniversalDocument) => void;
    /// Pagina mostrata: serve al pannello per modificare layout, righe e
    /// colonne della pagina corrente.
    onPageChange?: (index: number) => void;
    /// Elemento selezionato: il pannello laterale dell'app mostra l'ispezione
    /// al posto della vecchia colonna di proprietà interna all'editor.
    onElementSelection?: (selection: UniversalElementSelection | null) => void;
    /// Clic su un pittogramma (elemento o cella): il pannello laterale apre la
    /// scheda Simbolo con le alternative, come nel testo libero.
    onSymbolRequest?: (elementId: string, cellId: string | null) => void;
    /// Maiuscole solo visive (come nell'editor di testo).
    uppercase?: boolean;
  }

  let {
    document: initial,
    path = null,
    readOnly = false,
    profile = null,
    onSave,
    onSavePlayer,
    onClose,
    onToast,
    onSpeak,
    onExport,
    canEdit = false,
    onOpen,
    onImport,
    onEdit,
    onToggleFullscreen,
    onOpenAi,
    onOpenStudio,
    caa,
    onEditorReady,
    onDirtyChange,
    embedded = false,
    onTitleChange,
    onDocumentChange,
    onPageChange,
    onElementSelection,
    onSymbolRequest,
    uppercase = false,
  }: Props = $props();

  let doc = $state<UniversalDocument>(clone(initial));
  let rendered = $state<RenderedDocument>(localRender(doc));
  let pageIndex = $state(0);
  let selectedId = $state<string | null>(null);
  let extraSelected = $state<string[]>([]);
  let selectedCellId = $state<string | null>(null);
  let mode = $state<"edit" | "play">(readOnly ? "play" : "edit");
  let lastReadOnly = readOnly;
  $effect(() => {
    if (readOnly === lastReadOnly) return;
    lastReadOnly = readOnly;
    mode = readOnly ? "play" : "edit";
  });
  let busy = $state(false);
  let saving = $state(false);
  let searchQuery = $state("");
  let searchResults = $state<Alternative[]>([]);
  let onlineResults = $state<Array<OnlineResult & { thumb?: string }>>([]);
  let onlineBusy = $state(false);
  let onlineNote = $state("");
  let searchBusy = $state(false);
  let history: UniversalDocument[] = [];
  let redoStack: UniversalDocument[] = [];
  let message = $state<{ id: string; text: string; svg: string | null }[]>([]);
  let pageHistory = $state<number[]>([]);
  let toggled = $state<string[]>([]);
  let feedback = $state<{ text: string; ok: boolean } | null>(null);
  let zoom = $state(0.78);
  let dirty = $state(false);
  let gridPanel = $state(false);
  let gridColumns = $state(4);
  let gridRows = $state(5);
  let gridWidthsText = $state("");
  let gridHeightsText = $state("");
  let pagesOpen = $state(true);
  let showRuler = $state(true);
  let marginPreview = $state<number | null>(null);
  let rulerSide = $state<"left" | "right" | "top" | "bottom" | null>(null);
  let rulerStart = 0;
  let rulerStartMargin = 0;
  let snapGuides = $state<{ x: number | null; y: number | null }>({ x: null, y: null });
  let paginating = false;

  let textEditor = $state<CaaEditor | undefined>(undefined);
  let textTokens = new Map<string, CaaToken[]>();
  /// Elemento di cui si sta modificando il testo CAA con il motore condiviso.
  let editingTextId = $state<string | null>(null);
  /// Cella di griglia di cui si sta modificando la frase CAA.
  let editingCellId = $state<string | null>(null);
  const textPages = $derived(
    ((doc.metadata as Record<string, unknown> | undefined)?.text_pages ?? {}) as Record<
      string,
      { content?: unknown; plain_text?: string }
    >
  );
  const isTextPage = (id: string | undefined): boolean => Boolean(id && textPages[id]);

  const page = $derived(rendered.pages[pageIndex] ?? rendered.pages[0]);
  const docPage = $derived(doc.pages[pageIndex] ?? doc.pages[0]);
  const selectedRendered = $derived(
    page?.elements.find((element) => element.id === selectedId) ?? null
  );
  const selectedElement = $derived(
    docPage?.elements.find((element) => element.id === selectedId) ?? null
  );
  const selectedCell = $derived(
    selectedElement?.grid?.cells.find((cell) => cell.id === selectedCellId) ?? null
  );
  const editingCell = $derived(
    selectedElement?.grid?.cells.find((cell) => cell.id === editingCellId) ?? null
  );
  const editingElement = $derived(
    docPage?.elements.find((element) => element.id === editingTextId) ?? null
  );

  // Una sola fonte per l'ispezione: l'editor non disegna più una colonna di
  // proprietà propria quando è incorporato nella shell. Il pannello laterale
  // riceve un'istantanea stabile, aggiornata solo quando cambia davvero
  // qualcosa, così trascinare un elemento non ricrea il pannello a ogni pixel.
  // Il segnale parte a fine gesto: aprire il pannello a metà trascinamento
  // cambierebbe la larghezza della scena e sposterebbe l'elemento sotto il
  // puntatore.
  let selectionSignature = "";

  function publishSelection() {
    const element = selectedElement;
    const renderedElement = selectedRendered;
    const cell = selectedCell;
    const pageNow = docPage;
    if (!element || !renderedElement || !pageNow) {
      if (selectionSignature) {
        selectionSignature = "";
        onElementSelection?.(null);
      }
      return;
    }
    const signature = JSON.stringify({
      id: element.id,
      text: element.text,
      spoken: element.spoken_text,
      x: element.x_mm,
      y: element.y_mm,
      w: element.w_mm,
      h: element.h_mm,
      rotation: element.rotation,
      z: element.z,
      visible: element.visible,
      locked: element.locked,
      opacity: element.opacity,
      style: element.style,
      cell: cell?.id ?? null,
      symbol: element.symbol_id,
      image: Boolean(element.image_asset),
    });
    if (signature === selectionSignature) return;
    selectionSignature = signature;
    onElementSelection?.({
      element,
      rendered: renderedElement,
      cell,
      is_text_page: isTextPage(pageNow.id),
      page_width_mm: page?.width_mm ?? 210,
      page_height_mm: page?.height_mm ?? 297,
    });
  }

  $effect(() => {
    // Lettura esplicita delle dipendenze: serve anche quando il gesto è in
    // corso e la pubblicazione viene rimandata.
    void selectedId;
    void doc;
    void rendered;
    void pageIndex;
    if (drag) return;
    publishSelection();
  });

  $effect(() => {
    onPageChange?.(pageIndex);
  });

  function buildTextElements(
    paragraphs: Array<{ pid: string; text: string }>,
    tokens: Map<string, CaaToken[]>,
    includeTitle: boolean
  ): UniversalElement[] {
    const landscape = (doc.page?.orientation ?? "portrait") === "landscape";
    const pageW = doc.page?.width_mm ?? (landscape ? 297 : 210);
    const pageH = doc.page?.height_mm ?? (landscape ? 210 : 297);
    const margin = doc.page?.margin_mm ?? 15;
    const usable = pageW - margin * 2;
    let seq = 0;
    const elements: UniversalElement[] = [];
    const mk = (
      kind: string,
      text: string,
      x: number,
      y: number,
      w: number,
      h: number,
      extra: Record<string, unknown> = {}
    ) =>
      ({
        id: `t-${Date.now().toString(36)}-${seq++}`,
        kind,
        text,
        spoken_text: text,
        concept_id: null,
        symbol_id: null,
        lemma: null,
        svg: null,
        image_asset: null,
        audio_asset: null,
        x_mm: x,
        y_mm: y,
        w_mm: w,
        h_mm: h,
        rotation: 0,
        z: 0,
        visible: true,
        locked: false,
        opacity: 1,
        row: 0,
        column: 0,
        row_span: 1,
        column_span: 1,
        style: {},
        action: null,
        link_page: null,
        group_id: null,
        children: [],
        grid: null,
        ...extra,
      }) as UniversalElement;
    let y = margin;
    if (includeTitle && doc.title.trim()) {
      elements.push(
        mk("text", doc.title, margin, y, usable, 12, {
          style: { font_size: 20, bold: true, color: "#172033" },
        })
      );
      y += 18;
    }
    for (const paragraph of paragraphs) {
      const text = paragraph.text.trim();
      if (!text) continue;
      const unique: CaaToken[] = [];
      for (const token of tokens.get(paragraph.pid) ?? []) {
        if ((!token.svg && !token.image) || !token.symbol_id || token.is_punct || token.hidden) continue;
        if (!unique.some((item) => item.symbol_id === token.symbol_id)) unique.push(token);
      }
      const shown = unique.slice(0, 8);
      if (y + (shown.length > 0 ? 24 : 0) + 16 > pageH - margin) break;
      let x = margin;
      for (const token of shown) {
        elements.push(
          mk("symbol", token.surface, x, y, 18, 18, {
            svg: token.svg,
            image_asset: token.image,
            symbol_id: token.symbol_id,
            concept_id: token.concept_id,
            lemma: token.lemma,
          })
        );
        elements.push(mk("text", token.surface, x, y + 18, 18, 5, { style: { font_size: 8 } }));
        x += 20;
      }
      if (shown.length > 0) y += 24;
      elements.push(
        mk("text", text, margin, y, usable, 14, {
          style: { font_size: 14, color: "#172033" },
        })
      );
      y += 20;
    }
    return elements;
  }

  /// Salva il contenuto del CaaEditor incorporato nella pagina di testo e
  /// ricostruisce gli elementi di fallback usati da miniature ed esportazioni.
  function syncTextPage() {
    const current = doc.pages[pageIndex];
    if (!current || !isTextPage(current.id) || !textEditor) return;
    const metadata = (doc.metadata ?? {}) as Record<string, unknown>;
    const pages = { ...((metadata.text_pages ?? {}) as Record<string, unknown>) };
    pages[current.id] = {
      content: textEditor.getDocJson(),
      plain_text: textEditor.getPlainText(),
      saved_at: new Date().toISOString(),
    };
    // Gli elementi di flusso (id `t-…`) sono la copia usata da miniature,
    // stampa ed esportazioni. Fumetti, immagini e forme inseriti dall'utente
    // non devono sparire ogni volta che il testo viene risincronizzato: prima
    // questa assegnazione li cancellava dopo 600 ms, per questo il fumetto
    // restava visibile solo nella miniatura per un istante.
    current.elements = mergeTextElements(
      buildTextElements(textEditor.getParagraphs(), textTokens, pageIndex === 0),
      current.elements
    );
    doc.metadata = { ...metadata, text_pages: pages };
    rendered = localRender(doc);
    onDocumentChange?.(doc);
  }

  let textSyncTimer: number | undefined;
  function scheduleTextSync() {
    if (textSyncTimer) window.clearTimeout(textSyncTimer);
    textSyncTimer = window.setTimeout(() => {
      textSyncTimer = undefined;
      syncTextPage();
    }, 600);
  }

  function selectPage(index: number) {
    if (index === pageIndex) return;
    syncTextPage();
    pageIndex = index;
  }

  async function addTextPage() {
    syncTextPage();
    const id = `text-${Date.now().toString(36)}`;
    const metadata = (doc.metadata ?? {}) as Record<string, unknown>;
    const pages = { ...((metadata.text_pages ?? {}) as Record<string, unknown>) };
    pages[id] = { content: null, plain_text: "" };
    doc.metadata = { ...metadata, text_pages: pages };
    await apply(
      [{ op: "add_page", id, title: "Pagina di testo", layout: "absolute" }],
      "Pagina di testo aggiunta."
    );
    const index = doc.pages.findIndex((item) => item.id === id);
    if (index >= 0) pageIndex = index;
  }

  let loadedTextPage = "";
  $effect(() => {
    const id = docPage?.id ?? "";
    if (!textEditor || !isTextPage(id) || loadedTextPage === id) return;
    loadedTextPage = id;
    const content = textPages[id]?.content;
    if (content) {
      textEditor.setDocJson(content);
      // Prime miniature ed export: i pittogrammi arrivano dopo l'analisi.
      window.setTimeout(() => syncTextPage(), 400);
    }
  });

  $effect(() => {
    onEditorReady?.(textEditor);
    return () => onEditorReady?.(undefined);
  });

  function clone<T>(value: T): T {
    return JSON.parse(JSON.stringify(value)) as T;
  }

  function pageDimensions(document: UniversalDocument): [number, number] {
    if (document.page.width_mm && document.page.height_mm) {
      return [document.page.width_mm, document.page.height_mm];
    }
    const presets: Record<string, [number, number]> = {
      A4: [210, 297],
      A3: [297, 420],
      A5: [148, 210],
      LETTER: [215.9, 279.4],
      "16:9": [338.7, 190.5],
      "4:3": [254, 190.5],
    };
    const [w, h] = presets[document.page.size.toUpperCase()] ?? [210, 297];
    return document.page.orientation === "landscape" ? [h, w] : [w, h];
  }

  function textHeight(element: UniversalElement): number {
    const font = Math.max(2, element.style.font_size ?? 4.2);
    const chars = Math.max(1, element.text.length);
    const lines = Math.max(1, Math.ceil(chars / 32));
    return Math.max(8, lines * font * 1.5 + (element.style.padding_mm ?? 0) * 2);
  }

  function axisSizes(total: number, gap: number, count: number, weights?: number[]) {
    const n = Math.max(1, count);
    const usable = Math.max(1, total - gap * Math.max(0, n - 1));
    const valid =
      Array.isArray(weights) &&
      weights.length === n &&
      weights.every((value) => Number.isFinite(value) && value > 0);
    const normalized = valid
      ? (weights as number[]).map((value) => value / (weights as number[]).reduce((a, b) => a + b, 0))
      : Array.from({ length: n }, () => 1 / n);
    const sizes: number[] = [];
    const offsets: number[] = [];
    let cursor = 0;
    for (let index = 0; index < n; index += 1) {
      const size =
        index === n - 1
          ? Math.max(1, usable - sizes.reduce((a, b) => a + b, 0))
          : Math.max(1, usable * normalized[index]);
      offsets.push(cursor);
      sizes.push(size);
      cursor += size + gap;
    }
    return { sizes, offsets };
  }

  type WrapZone = { x0: number; x1: number; bottom: number; left: boolean };

  class FlowLayout {
    left: number;
    right: number;
    gap: number;
    x: number;
    y: number;
    lineH = 0;
    zone: WrapZone | null = null;
    constructor(left: number, right: number, top: number, gap: number) {
      this.left = left;
      this.right = right;
      this.gap = Math.max(0, gap);
      this.x = left;
      this.y = top;
    }
    private area(): [number, number] {
      if (this.zone && this.y < this.zone.bottom) {
        if (this.zone.left) {
          return [this.zone.x1 + this.gap, Math.max(8, this.right - this.zone.x1 - this.gap)];
        }
        return [this.left, Math.max(8, this.zone.x0 - this.gap - this.left)];
      }
      return [this.left, Math.max(8, this.right - this.left)];
    }
    private clearZone(): void {
      if (this.zone && this.y >= this.zone.bottom) this.zone = null;
    }
    inline(width: number, height: number): [number, number] {
      this.clearZone();
      let [areaX, areaW] = this.area();
      let w = Math.min(width, areaW);
      if (this.x < areaX || this.x + w > areaX + areaW + 0.01) {
        this.x = areaX;
        if (this.lineH > 0) {
          this.y += this.lineH + this.gap;
          this.lineH = 0;
        }
      }
      [areaX] = this.area();
      if (this.x < areaX) this.x = areaX;
      const placed: [number, number] = [this.x, this.y];
      this.x += w + this.gap;
      this.lineH = Math.max(this.lineH, height);
      return placed;
    }
    block(width: number, height: number, textlike: boolean): [number, number] {
      this.clearZone();
      if (textlike && this.zone && this.y < this.zone.bottom) {
        const [areaX, areaW] = this.area();
        const placed: [number, number] = [areaX, this.y];
        this.x = areaX + areaW + this.gap;
        this.lineH = 0;
        this.y += height + this.gap;
        return placed;
      }
      if (this.zone && this.y < this.zone.bottom) this.y = this.zone.bottom + this.gap;
      this.zone = null;
      const placed: [number, number] = [this.left, this.y];
      this.x = this.left + Math.min(width, this.right - this.left) + this.gap;
      this.lineH = 0;
      this.y += height + this.gap;
      return placed;
    }
    wrap(width: number, height: number, left: boolean): [number, number] {
      if (this.zone && this.y < this.zone.bottom) this.y = this.zone.bottom + this.gap;
      const w = Math.min(Math.max(width, 8), Math.max(8, this.right - this.left));
      const x = left ? this.left : this.right - w;
      const y = this.y;
      this.zone = { x0: x, x1: x + w, bottom: y + height, left };
      this.x = this.left;
      this.lineH = 0;
      return [x, y];
    }
  }

  function elementFlowHeight(element: UniversalElement): number {
    if (element.h_mm > 0) return element.h_mm;
    if (["symbol", "button", "grid", "image"].includes(element.kind)) return 28;
    return textHeight(element);
  }

  function elementIsTextlike(element: UniversalElement): boolean {
    return (
      element.text.trim().length > 0 &&
      !element.svg &&
      !element.image_asset &&
      !["grid", "table", "image", "button", "symbol"].includes(element.kind)
    );
  }

  function localRender(document: UniversalDocument): RenderedDocument {
    const [pageW, pageH] = pageDimensions(document);
    const margin = Math.max(0, document.page.margin_mm);
    const warnings: string[] = [];
    const pages = document.pages.map((item, index) => {
      let flowY = margin + (item.flow.padding_mm ?? 0);
      const flowW = pageW - 2 * margin - 2 * (item.flow.padding_mm ?? 0);
      const flow = new FlowLayout(
        margin + (item.flow.padding_mm ?? 0),
        pageW - margin - (item.flow.padding_mm ?? 0),
        flowY,
        item.flow.gap_mm ?? 3
      );
      const textPagesMeta = (document.metadata as Record<string, unknown> | undefined)?.text_pages as
        | Record<string, unknown>
        | undefined;
      const pageIsText = Boolean(textPagesMeta?.[item.id]);
      const columnW =
        item.flow.columns > 1
          ? (flowW - item.flow.gap_mm * (item.flow.columns - 1)) / item.flow.columns
          : flowW;
      const gridW =
        (pageW - 2 * margin - item.grid.gap_mm * Math.max(0, item.grid.columns - 1)) /
        Math.max(1, item.grid.columns);
      const gridH =
        (pageH - 2 * margin - item.grid.gap_mm * Math.max(0, item.grid.rows - 1)) /
        Math.max(1, item.grid.rows);
      const elements = [...item.elements]
        .sort((a, b) => a.z - b.z || a.y_mm - b.y_mm || a.x_mm - b.x_mm)
        .map((element) => {
          let x = element.x_mm;
          let y = element.y_mm;
          let w = element.w_mm;
          let h = element.h_mm;
          const disposition = element.disposition ?? "";
          if (!pageIsText && disposition) {
            const height = elementFlowHeight(element);
            if (disposition === "inline") {
              w =
                element.w_mm > 0
                  ? Math.min(element.w_mm, flowW)
                  : elementIsTextlike(element)
                    ? flowW
                    : Math.min(40, flowW);
              h = height;
              [x, y] = flow.inline(w, h);
            } else if (disposition === "block") {
              w = element.w_mm > 0 ? Math.min(element.w_mm, flowW) : flowW;
              h = height;
              [x, y] = flow.block(w, h, elementIsTextlike(element));
            } else if (disposition === "wrap_left" || disposition === "wrap_right") {
              const natural = element.w_mm > 0 ? element.w_mm : Math.max(30, flowW * 0.42);
              w = Math.min(Math.max(natural, 20), Math.max(24, flowW * 0.62));
              h = height;
              [x, y] = flow.wrap(w, h, disposition === "wrap_left");
            } else {
              // front / behind / free: posizione propria, fuori dal flusso.
              w = element.w_mm > 0 ? element.w_mm : Math.min(120, pageW - 2 * margin);
              h = element.h_mm > 0 ? element.h_mm : textHeight(element);
              x = element.x_mm;
              y = element.y_mm;
            }
          } else if (item.layout === "flow") {
            w = element.w_mm > 0 ? Math.min(element.w_mm, flowW) : columnW;
            h =
              element.h_mm > 0
                ? element.h_mm
                : ["symbol", "button", "grid", "image"].includes(element.kind)
                  ? 28
                  : textHeight(element);
            const column = Math.min(element.column, Math.max(0, item.flow.columns - 1));
            x = margin + column * (columnW + item.flow.gap_mm);
            y = flowY;
            flowY += h + item.flow.gap_mm;
          } else if (item.layout === "grid") {
            const column = Math.min(element.column, Math.max(0, item.grid.columns - 1));
            const row = Math.min(element.row, Math.max(0, item.grid.rows - 1));
            const spanCols = Math.max(1, Math.min(element.column_span, item.grid.columns - column));
            const spanRows = Math.max(1, Math.min(element.row_span, item.grid.rows - row));
            w = gridW * spanCols + item.grid.gap_mm * Math.max(0, spanCols - 1);
            h = gridH * spanRows + item.grid.gap_mm * Math.max(0, spanRows - 1);
            x = margin + column * (gridW + item.grid.gap_mm);
            y = margin + row * (gridH + item.grid.gap_mm);
          } else if (w <= 0) {
            w = Math.min(120, pageW - 2 * margin);
            h = h <= 0 ? textHeight(element) : h;
          }
          const cells = element.grid
            ? (() => {
                const columns = Math.max(1, element.grid.columns);
                const rows = Math.max(1, element.grid.rows);
                const gap = Math.max(0, element.grid.gap_mm);
                const col = axisSizes(w, gap, columns, element.grid.column_widths);
                const row = axisSizes(h, gap, rows, element.grid.row_heights);
                const legacy =
                  element.grid.cells.length > 1 &&
                  element.grid.cells.every(
                    (cell) => (cell.row ?? 0) === 0 && (cell.column ?? 0) === 0
                  );
                return element.grid.cells.map((cell, index) => {
                  const cellRow = legacy ? Math.floor(index / columns) : (cell.row ?? 0);
                  const cellColumn = legacy ? index % columns : (cell.column ?? 0);
                  const columnSpan = Math.max(
                    1,
                    Math.min(cell.column_span ?? 1, columns - cellColumn)
                  );
                  const rowSpan = Math.max(1, Math.min(cell.row_span ?? 1, rows - cellRow));
                  const width =
                    col.sizes.slice(cellColumn, cellColumn + columnSpan).reduce((a, b) => a + b, 0) +
                    gap * Math.max(0, columnSpan - 1);
                  const height =
                    row.sizes.slice(cellRow, cellRow + rowSpan).reduce((a, b) => a + b, 0) +
                    gap * Math.max(0, rowSpan - 1);
                  return {
                    id: cell.id,
                    row: cellRow,
                    column: cellColumn,
                    row_span: rowSpan,
                    column_span: columnSpan,
                    text: cell.text,
                    spoken_text: cell.spoken_text || cell.text,
                    symbol_id: cell.symbol_id ?? null,
                    concept_id: cell.concept_id ?? null,
                    svg: cell.svg ?? null,
                    image_asset: cell.image_asset ?? null,
                    action: cell.action ?? null,
                    hidden: Boolean(cell.hidden),
                    x_mm: col.offsets[cellColumn] ?? 0,
                    y_mm: row.offsets[cellRow] ?? 0,
                    w_mm: width,
                    h_mm: height,
                    style: cell.style ?? {},
                    metadata:
                      cell.metadata && typeof cell.metadata === "object"
                        ? (cell.metadata as Record<string, unknown>)
                        : null,
                  };
                });
              })()
            : [];
          return {
            id: element.id,
            kind: element.kind,
            text: element.text,
            spoken_text: element.spoken_text || element.text,
            concept_id: element.concept_id ?? null,
            symbol_id: element.symbol_id ?? null,
            svg: element.svg ?? null,
            image_asset: element.image_asset ?? null,
            audio_asset: element.audio_asset ?? null,
            x_mm: x,
            y_mm: y,
            w_mm: Math.max(0, w),
            h_mm: Math.max(0, h),
            rotation: element.rotation,
            z: element.z,
            visible: element.visible,
            locked: element.locked,
            opacity: element.opacity,
            style: element.style ?? {},
            disposition: element.disposition ?? "",
            action: element.action ?? null,
            link_page: element.link_page ?? null,
            group_id: element.group_id ?? null,
            cells,
            accessibility: element.accessibility ?? {},
            metadata:
              element.metadata && typeof element.metadata === "object"
                ? (element.metadata as Record<string, unknown>)
                : {},
          };
        });
      return {
        id: item.id,
        title: item.title,
        index,
        layout: item.layout,
        width_mm: pageW,
        height_mm: pageH,
        background: item.background || document.page.background,
        is_home: item.is_home,
        elements,
      } as RenderedPage;
    });
    return {
      id: document.id,
      title: document.title,
      kind: document.kind,
      schema_version: document.schema_version,
      pages,
      warnings,
    };
  }

  async function refresh(document: UniversalDocument) {
    doc = clone(document);
    rendered = localRender(doc);
    if (isTauri()) {
      try {
        rendered = await api.documentRender(doc);
      } catch (error) {
        onToast(error instanceof Error ? error.message : String(error), "error");
      }
    }
    dirty = true;
  }

  /// Porta su una nuova pagina gli elementi che superano il margine
  /// inferiore. È voluto per i documenti a flusso: l'utente continua a
  /// scrivere e il foglio successivo nasce senza un comando manuale.
  async function autoPaginateFlow() {
    if (mode !== "edit" || readOnly || paginating || !docPage || docPage.layout !== "flow") {
      return;
    }
    const current = rendered.pages[pageIndex];
    if (!current || current.elements.length < 2) return;
    const limit = current.height_mm - doc.page.margin_mm + 0.5;
    const overflow = current.elements.filter(
      (element) => element.y_mm + element.h_mm > limit
    );
    if (overflow.length === 0) return;
    const firstId = overflow[0].id;
    const start = docPage.elements.findIndex((element) => element.id === firstId);
    if (start <= 0) return;
    const moving = docPage.elements.slice(start);
    const newPageId = `page-${Date.now().toString(36)}`;
    const commands: UniversalCommand[] = [
      {
        op: "add_page",
        id: newPageId,
        title: `${docPage.title} · ${doc.pages.length + 1}`,
        layout: "flow",
        after: docPage.id,
      },
      ...moving.map((element) => ({
        op: "remove_element",
        page: docPage.id,
        id: element.id,
      })),
      ...moving.map((element) => ({
        op: "add_element",
        page: newPageId,
        element: { ...element, x_mm: 0, y_mm: 0, row: 0, column: 0 },
      })),
    ];
    paginating = true;
    try {
      await apply(commands, "Spazio esaurito: creata una nuova pagina.");
      await tick();
      const nextIndex = doc.pages.findIndex((item) => item.id === newPageId);
      if (nextIndex >= 0) pageIndex = nextIndex;
    } finally {
      paginating = false;
    }
  }

  async function apply(commands: UniversalCommand[], label: string) {
    if (commands.length === 0) return;
    if (!isTauri()) {
      onToast("La modifica del documento richiede l'app desktop.", "warn");
      return;
    }
    busy = true;
    try {
      const result = await api.documentApply(doc, commands);
      history = [...history.slice(-49), clone(doc)];
      redoStack = [];
      doc = result.document;
      rendered = result.rendered ?? localRender(result.document);
      onDocumentChange?.(doc);
      dirty = true;
      onDirtyChange?.(true);
      if (!doc.pages[pageIndex]) pageIndex = Math.max(0, doc.pages.length - 1);
      if (label) onToast(label, "info");
      if (!commands.some((command) => command.op === "add_page")) {
        setTimeout(() => void autoPaginateFlow(), 0);
      }
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      busy = false;
    }
  }

  /// Inserisce un'immagine generata (SVG con PNG incorporato) nella pagina
  /// corrente come elemento modificabile. Usata dal pannello IA.
  export async function insertImageElement(svg: string, label: string) {
    const page = doc.pages[pageIndex] ?? doc.pages[0];
    if (!page) {
      onToast("Apri una pagina del documento prima di inserire l'immagine.", "warn");
      return;
    }
    const id = `ai-${Date.now().toString(36)}-${Math.floor(Math.random() * 1000)}`;
    const offset = (page.elements.length % 6) * 6;
    const element = {
      id,
      kind: "image",
      text: label,
      spoken_text: label,
      svg,
      x_mm: 20 + offset,
      y_mm: 20 + offset,
      w_mm: 70,
      h_mm: 70,
    };
    await apply(
      [{ op: "add_element", page: page.id, element } as UniversalCommand],
      "Immagine generata inserita."
    );
    selectedId = id;
    selectedCellId = null;
  }

  /// Inserisce un foglio generato (pregrafismo) con le sue dimensioni reali,
  /// come immagine modificabile e stampabile.
  export async function insertSheetElement(
    svg: string,
    widthMm: number,
    heightMm: number,
    label: string
  ) {
    const page = doc.pages[pageIndex] ?? doc.pages[0];
    if (!page) {
      onToast("Apri una pagina del documento prima di inserire il foglio.", "warn");
      return;
    }
    const id = `trace-${Date.now().toString(36)}`;
    const maxWidth = Math.max(40, (doc.page.width_mm ?? 210) - 20);
    const maxHeight = Math.max(40, (doc.page.height_mm ?? 297) - 20);
    const scale = Math.min(1, maxWidth / Math.max(1, widthMm), maxHeight / Math.max(1, heightMm));
    const element = {
      id,
      kind: "image",
      text: label,
      spoken_text: label,
      svg,
      x_mm: 10,
      y_mm: 10,
      w_mm: Math.max(20, widthMm * scale),
      h_mm: Math.max(20, heightMm * scale),
    };
    await apply(
      [{ op: "add_element", page: page.id, element } as UniversalCommand],
      "Foglio di pregrafismo inserito."
    );
    selectedId = id;
    selectedCellId = null;
  }

  function undo() {
    const previous = history.pop();
    if (!previous) return;
    redoStack = [...redoStack, clone(doc)];
    doc = previous;
    rendered = localRender(doc);
    dirty = true;
    selectedId = null;
    selectedCellId = null;
    if (isTauri()) void api.documentRender(doc).then((value) => (rendered = value)).catch(() => {});
  }

  function redo() {
    const next = redoStack.pop();
    if (!next) return;
    history = [...history, clone(doc)];
    doc = next;
    rendered = localRender(doc);
    dirty = true;
    selectedId = null;
    selectedCellId = null;
    if (isTauri()) void api.documentRender(doc).then((value) => (rendered = value)).catch(() => {});
  }

  function updateElement(id: string, patch: Record<string, unknown>) {
    if (!docPage) return;
    // Silenzioso: la modifica è già visibile nell'anteprima. Un toast per ogni
    // ritocco coprirebbe il lavoro e sembrerebbe una conferma richiesta.
    void apply([{ op: "update_element", page: docPage.id, id, patch }], "");
  }

  function setStyle(patch: Partial<UniversalStyle>) {
    if (!docPage || !selectedId) return;
    void apply([{ op: "set_style", page: docPage.id, id: selectedId, style: patch }], "");
  }

  function setAction(patch: Record<string, unknown> | null) {
    if (!docPage || !selectedId) return;
    const action = patch
      ? { type: String(patch.type ?? "append"), target_page: (patch.target_page as string) || null, target_document: null, url: null }
      : null;
    void apply(
      [{ op: "set_action", page: docPage.id, id: selectedId, action }],
      ""
    );
  }

  function setText(value: string, spoken = false) {
    if (!docPage || !selectedId) return;
    if (spoken) {
      updateElement(selectedId, { spoken_text: value });
    } else {
      void apply(
        [{ op: "set_text", page: docPage.id, id: selectedId, text: value }],
        ""
      );
    }
  }

  /// Apre la modifica del testo CAA di un elemento con lo stesso motore del
  /// word processor: cursore dentro le parole, selezione, alternative, blocco.
  export function inspectorEditCaA(id?: string) {
    const targetId = id ?? selectedId;
    if (!targetId) return;
    const target = docPage?.elements.find((element) => element.id === targetId);
    if (!target) return;
    if (target.locked) {
      onToast("Elemento bloccato: sbloccalo per modificarne il testo.", "warn");
      return;
    }
    if (!supportsCaaText(target.kind)) {
      onToast(`«${target.kind}» non contiene testo CAA modificabile.`, "warn");
      return;
    }
    editingTextId = targetId;
  }

  /// Salva testo, pittogrammi scelti e struttura ProseMirror nell'elemento.
  /// Il testo resta la fonte autorevole; i token completano la resa visiva.
  function applyElementText(value: {
    text: string;
    tokens: StoredCaaToken[];
    docJson: unknown;
  }) {
    const target = editingElement;
    if (!docPage || !target) return;
    const current =
      target.metadata && typeof target.metadata === "object"
        ? ({ ...(target.metadata as Record<string, unknown>) } as Record<string, unknown>)
        : {};
    if (value.tokens.length) {
      current.caa = { doc_json: value.docJson, plain_text: value.text, tokens: value.tokens };
    } else {
      delete current.caa;
    }
    void apply(
      [
        {
          op: "update_element",
          page: docPage.id,
          id: target.id,
          patch: {
            text: value.text,
            spoken_text: target.spoken_text?.trim() ? target.spoken_text : value.text,
            metadata: current,
          },
        },
      ],
      "Testo e pittogrammi aggiornati."
    );
    editingTextId = null;
  }

  /// Apre la frase CAA di una cella con lo stesso editor condiviso.
  function editCellCaA(cellId: string) {
    if (mode !== "edit" || !selectedElement?.grid) return;
    if (selectedElement.locked) {
      onToast("Elemento bloccato: sbloccalo per modificarne le celle.", "warn");
      return;
    }
    selectedCellId = cellId;
    editingCellId = cellId;
  }

  /// Salva testo, token e struttura ProseMirror nella cella. Gli altri campi
  /// della cella (pittogramma singolo, stile, azione) restano intatti.
  function applyCellText(value: {
    text: string;
    tokens: StoredCaaToken[];
    docJson: unknown;
  }) {
    if (!docPage || !selectedElement?.grid || !editingCell) return;
    const cells = selectedElement.grid.cells.map((cell) => {
      if (cell.id !== editingCell.id) return cell;
      const metadata =
        cell.metadata && typeof cell.metadata === "object"
          ? ({ ...(cell.metadata as Record<string, unknown>) } as Record<string, unknown>)
          : {};
      if (value.tokens.length) {
        metadata.caa = { doc_json: value.docJson, plain_text: value.text, tokens: value.tokens };
      } else {
        delete metadata.caa;
      }
      return {
        ...cell,
        text: value.text,
        spoken_text: cell.spoken_text?.trim() ? cell.spoken_text : value.text,
        metadata,
      };
    });
    updateElement(selectedElement.id, {
      grid: { ...selectedElement.grid, cells },
    });
    editingCellId = null;
  }

  /// Riapre la modifica con il cursore sulla parola del pittogramma cliccato.
  function editSymbolOf(element: RenderedElement) {
    if (mode !== "edit") return;
    editingTextId = element.id;
  }

  function setNumber(field: string, value: number) {
    if (!docPage || !selectedId) return;
    if (field === "x_mm" || field === "y_mm") {
      void apply(
        [{
          op: "set_position",
          page: docPage.id,
          id: selectedId,
          x_mm: field === "x_mm" ? value : selectedElement?.x_mm ?? 0,
          y_mm: field === "y_mm" ? value : selectedElement?.y_mm ?? 0,
        }],
        "Posizione aggiornata."
      );
    } else if (field === "w_mm" || field === "h_mm") {
      void apply(
        [{
          op: "set_size",
          page: docPage.id,
          id: selectedId,
          w_mm: field === "w_mm" ? value : selectedElement?.w_mm ?? 10,
          h_mm: field === "h_mm" ? value : selectedElement?.h_mm ?? 10,
        }],
        "Dimensioni aggiornate."
      );
    } else if (field === "rotation") {
      void apply(
        [{ op: "set_rotation", page: docPage.id, id: selectedId, rotation: value }],
        "Rotazione aggiornata."
      );
    } else {
      updateElement(selectedId, { [field]: value });
    }
  }

  function addElement(kind: string) {
    if (!docPage) return;
    if (kind === "table") {
      addTable();
      return;
    }
    const id = `el-${Date.now().toString(36)}`;
    const element: UniversalElement = {
      id,
      kind,
      text:
        kind === "text" || kind === "text_box" || kind === "button"
          ? "Nuovo testo"
          : kind === "speech"
            ? "Nuovo fumetto"
            : "",
      spoken_text: "",
      x_mm: Math.max(5, (doc.page.margin_mm || 12) + 5),
      y_mm: Math.max(5, (doc.page.margin_mm || 12) + 5),
      w_mm: kind === "button" ? 45 : kind === "speech" ? 72 : 80,
      h_mm: kind === "button" ? 22 : kind === "shape" ? 30 : kind === "speech" ? 28 : 20,
      rotation: 0,
      z: docPage.elements.length,
      visible: true,
      locked: false,
      opacity: 1,
      row: 0,
      column: 0,
      row_span: 1,
      column_span: 1,
      style:
        kind === "button"
          ? {
              font_size: 4.5,
              border_width: 0,
              border_color: "#1f5bd8",
              background: "#1f5bd8",
              color: "#ffffff",
              border_radius: 6,
              align: "center",
            }
          : kind === "speech"
            ? {
                font_size: 4.5,
                border_width: 0.7,
                border_color: "#172033",
                border_radius: 6,
                background: "#ffffff",
                align: "center",
              }
            : { font_size: 4.5 },
      disposition: "",
      action: kind === "button" ? { type: "speak", target_page: null } : null,
      link_page: null,
      group_id: null,
      children: [],
      grid: null,
      accessibility: { label: "" },
      metadata: null,
    };
    void apply([{ op: "add_element", page: docPage.id, element }], "Elemento aggiunto.");
    selectedId = id;
  }

  /// Doppio clic in un'area libera di una pagina non testuale: crea un vero
  /// paragrafo di testo nel punto scelto, non una casella invisibile sopra la
  /// pagina. Nelle pagine di flusso il paragrafo entra in coda al flusso.
  async function pageDoubleClick(event: MouseEvent) {
    if (mode !== "edit" || readOnly || !docPage) return;
    if (isTextPage(docPage.id)) return;
    const target = event.target as HTMLElement | null;
    if (!target || target.closest(".uel")) return;
    const host = event.currentTarget as HTMLElement;
    const rect = host.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) return;
    const pageW = rendered.pages[pageIndex]?.width_mm ?? 210;
    const pageH = rendered.pages[pageIndex]?.height_mm ?? 297;
    const margin = doc.page.margin_mm || 12;
    const x = Math.min(
      Math.max(margin, ((event.clientX - rect.left) / rect.width) * pageW),
      Math.max(margin, pageW - margin - 40)
    );
    const y = Math.min(
      Math.max(margin, ((event.clientY - rect.top) / rect.height) * pageH),
      Math.max(margin, pageH - margin - 12)
    );
    const inFlow = docPage.layout === "flow";
    const id = `txt-${Date.now().toString(36)}`;
    const element: UniversalElement = {
      id,
      kind: "text",
      text: "",
      spoken_text: "",
      x_mm: inFlow ? margin : x,
      y_mm: inFlow ? margin : y,
      w_mm: inFlow ? Math.max(40, pageW - margin * 2) : Math.max(40, pageW - margin - x),
      h_mm: 18,
      rotation: 0,
      z: docPage.elements.length,
      visible: true,
      locked: false,
      opacity: 1,
      row: 0,
      column: 0,
      row_span: 1,
      column_span: 1,
      style: { font_size: 4.5 },
      disposition: inFlow ? "block" : "free",
      action: null,
      link_page: null,
      group_id: null,
      children: [],
      grid: null,
      accessibility: { label: "" },
      metadata: null,
    };
    await apply([{ op: "add_element", page: docPage.id, element }], "");
    selectedId = id;
    selectedCellId = null;
    inspectorEditCaA(id);
  }

  /// Tabella modificabile: usa la stessa griglia di celle dell'editor, con
  /// la prima riga come intestazione. Non è un elemento decorativo separato.
  function addTable() {
    if (!docPage) return;
    const columns = 3;
    const rows = 2;
    const id = `table-${Date.now().toString(36)}`;
    const margin = doc.page.margin_mm || 12;
    const [pageW] = pageDimensions(doc);
    const cells = Array.from({ length: columns * rows }, (_, index) => {
      const row = Math.floor(index / columns);
      return {
        id: `${id}-c${index + 1}`,
        row,
        column: index % columns,
        row_span: 1,
        column_span: 1,
        text: "",
        spoken_text: "",
        style: {
          font_size: 4.2,
          border_width: 0.25,
          border_color: "#94a3b8",
          padding_mm: 1.2,
          background: row === 0 ? "#eef2f8" : "#ffffff",
          bold: row === 0,
          align: "left",
        },
        hidden: false,
        metadata: null,
      };
    });
    const element: UniversalElement = {
      id,
      kind: "table",
      text: "",
      spoken_text: "",
      x_mm: margin,
      y_mm: margin,
      w_mm: Math.max(60, pageW - 2 * margin),
      h_mm: 40,
      rotation: 0,
      z: docPage.elements.length,
      visible: true,
      locked: false,
      opacity: 1,
      row: 0,
      column: 0,
      row_span: 1,
      column_span: 1,
      style: { font_size: 4.2, border_width: 0.25, border_color: "#94a3b8" },
      disposition: "block",
      action: null,
      link_page: null,
      group_id: null,
      children: [],
      grid: { columns, rows, gap_mm: 1, column_widths: [], row_heights: [], cells },
      accessibility: { label: "Tabella" },
      metadata: null,
    };
    void apply([{ op: "add_element", page: docPage.id, element }], "Tabella 3×2 creata.");
    selectedId = id;
  }

  function addGrid() {
    if (!docPage) return;
    const columns = Math.max(1, Math.min(20, Math.round(gridColumns)));
    const rows = Math.max(1, Math.min(40, Math.round(gridRows)));
    const id = `grid-${Date.now().toString(36)}`;
    const [pageW, pageH] = pageDimensions(doc);
    const margin = doc.page.margin_mm || 12;
    void apply(
      [{
        op: "create_grid",
        page: docPage.id,
        id,
        columns,
        rows,
        gap_mm: 3,
        x_mm: margin,
        y_mm: margin,
        w_mm: Math.max(40, pageW - 2 * margin),
        h_mm: Math.max(40, pageH - 2 * margin),
      }],
      `Griglia ${columns}×${rows}: ${columns * rows} celle create.`
    );
    selectedId = id;
    gridPanel = false;
  }

  function resizeGrid(deltaRows: number, deltaColumns: number) {
    if (!selectedElement?.grid || !docPage) return;
    const columns = Math.max(1, selectedElement.grid.columns + deltaColumns);
    const rows = Math.max(1, selectedElement.grid.rows + deltaRows);
    void apply(
      [{
        op: "update_grid",
        page: docPage.id,
        id: selectedElement.id,
        columns,
        rows,
        cells: null,
      }],
      `Griglia ${columns}×${rows}.`
    );
  }

  function setGridSize(columns?: number, rows?: number) {
    if (!selectedElement?.grid || !docPage) return;
    const nextColumns = Math.max(
      1,
      Math.min(20, Math.round(columns ?? selectedElement.grid.columns))
    );
    const nextRows = Math.max(1, Math.min(40, Math.round(rows ?? selectedElement.grid.rows)));
    void apply(
      [{
        op: "update_grid",
        page: docPage.id,
        id: selectedElement.id,
        columns: nextColumns,
        rows: nextRows,
        cells: null,
      }],
      `Griglia ${nextColumns}×${nextRows}.`
    );
  }

  function mergeCell(direction: "right" | "down") {
    if (!selectedElement?.grid || !selectedCell || !docPage) return;
    void apply(
      [{
        op: "merge_grid_cell",
        page: docPage.id,
        id: selectedElement.id,
        cell_id: selectedCell.id,
        direction,
      }],
      "Celle unite."
    );
  }

  function splitCell() {
    if (!selectedElement?.grid || !selectedCell || !docPage) return;
    void apply(
      [{
        op: "split_grid_cell",
        page: docPage.id,
        id: selectedElement.id,
        cell_id: selectedCell.id,
      }],
      "Cella divisa."
    );
  }

  function applyGridSizes() {
    if (!selectedElement?.grid || !docPage) return;
    const widths = gridWidthsText
      .split(",")
      .map((value) => Number(value.trim()))
      .filter((value) => Number.isFinite(value) && value > 0);
    const heights = gridHeightsText
      .split(",")
      .map((value) => Number(value.trim()))
      .filter((value) => Number.isFinite(value) && value > 0);
    void apply(
      [{
        op: "set_grid_sizes",
        page: docPage.id,
        id: selectedElement.id,
        column_widths: widths.length === selectedElement.grid.columns ? widths : null,
        row_heights: heights.length === selectedElement.grid.rows ? heights : null,
      }],
      "Dimensioni della griglia aggiornate."
    );
  }

  function setCellText(cellId: string, value: string) {
    if (!selectedElement?.grid || !docPage) return;
    const cells = selectedElement.grid.cells.map((cell) =>
      cell.id === cellId ? { ...cell, text: value } : cell
    );
    updateElement(selectedElement.id, {
      grid: { ...selectedElement.grid, cells },
    });
  }

  function clearGridCells() {
    if (!selectedElement?.grid || !docPage) return;
    if (!confirm("Svuotare tutte le celle della griglia?")) return;
    const cells = selectedElement.grid.cells.map((cell) => ({
      ...cell,
      text: "",
      spoken_text: "",
      symbol_id: null,
      concept_id: null,
      svg: null,
      action: null,
    }));
    updateElement(selectedElement.id, {
      grid: { ...selectedElement.grid, cells },
    });
  }

  async function searchSymbol() {
    const query = searchQuery.trim();
    if (!query) return;
    if (!isTauri()) {
      onToast("La ricerca dei pittogrammi richiede l'app desktop.", "warn");
      return;
    }
    searchBusy = true;
    onlineResults = [];
    onlineNote = "";
    try {
      const result = await api.searchSymbols(query, 18, 0, null, false);
      searchResults = result.items;
      if (result.items.length === 0) await searchOnlineSymbols(query);
    } finally {
      searchBusy = false;
    }
  }

  async function searchOnlineSymbols(query: string) {
    onlineBusy = true;
    try {
      const response = await api.searchOnlinePictograms([query], false);
      const items = response.results.slice(0, 12);
      const withThumbs = await Promise.all(
        items.map(async (item) => {
          let thumb: string | undefined;
          try {
            thumb = await api.onlineThumbnail(item.thumbnail_url ?? item.image_url);
          } catch {
            thumb = undefined;
          }
          return { ...item, thumb };
        })
      );
      onlineResults = withThumbs;
      onlineNote =
        response.warnings[0] ??
        (withThumbs.length > 0
          ? "Risultati da API pubblica: scegli un simbolo per salvarlo offline."
          : "Nessun risultato online.");
    } catch (error) {
      onlineNote = error instanceof Error ? error.message : String(error);
    } finally {
      onlineBusy = false;
    }
  }

  async function useOnlineResult(result: OnlineResult) {
    if (!isTauri() || !selectedId) return;
    onlineBusy = true;
    try {
      const alternative = await api.saveOnlinePictogram(
        result,
        searchQuery.trim() || result.title,
        "Ricerca online"
      );
      applySymbol(alternative);
      onlineResults = [];
      onlineNote = "";
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      onlineBusy = false;
    }
  }

  function applySymbol(alternative: Alternative) {
    if (!docPage || !selectedId || !selectedElement) return;
    if (selectedCell && selectedElement.grid) {
      const cells = clone(selectedElement.grid.cells);
      const target = cells.find((cell) => cell.id === selectedCell.id);
      if (target) {
        target.symbol_id = alternative.symbol_id;
        target.concept_id = alternative.concept_id;
        target.svg = alternative.svg;
        target.image_asset = alternative.image ?? null;
        if (!target.text.trim()) target.text = alternative.label;
        if (!target.spoken_text.trim()) target.spoken_text = alternative.label;
        updateElement(selectedElement.id, {
          grid: { ...selectedElement.grid, cells },
        });
      }
    } else if (selectedElement.kind === "symbol") {
      void apply(
        [{
          op: "set_symbol",
          page: docPage.id,
          id: selectedId,
          symbol_id: alternative.symbol_id,
          concept_id: alternative.concept_id,
          svg: alternative.svg,
          image_asset: alternative.image ?? null,
          label: alternative.label,
        }],
        "Pittogramma sostituito."
      );
    } else {
      // Su pulsanti, forme, fumetti e caselle la sostituzione non deve
      // cambiare il tipo dell'elemento né la sua azione: si aggiornano solo i
      // campi del pittogramma e il testo resta quello scritto dall'utente.
      updateElement(selectedId, {
        symbol_id: alternative.symbol_id,
        concept_id: alternative.concept_id,
        svg: alternative.svg,
        image_asset: alternative.image ?? null,
        ...(selectedElement.text.trim()
          ? {}
          : { text: alternative.label, spoken_text: selectedElement.spoken_text || alternative.label }),
      });
    }
    searchResults = [];
    searchQuery = "";
  }

  const thumbPct = (value: number, total: number) => (total > 0 ? (value / total) * 100 : 0);

  function addPage() {
    const id = `page-${Date.now().toString(36)}`;
    void apply(
      [{
        op: "add_page",
        id,
        title: `Pagina ${doc.pages.length + 1}`,
        layout: docPage?.layout ?? "flow",
      }],
      "Pagina aggiunta."
    );
    pageIndex = doc.pages.length;
  }

  function duplicatePage() {
    if (!docPage) return;
    syncTextPage();
    const newPageId = `page-${Date.now().toString(36)}`;
    const commands: UniversalCommand[] = [{
      op: "add_page",
      id: newPageId,
      title: `${docPage.title} (copia)`,
      layout: docPage.layout,
    }];
    for (const element of docPage.elements) {
      commands.push({
        op: "add_element",
        page: newPageId,
        element: { ...element, id: `${element.id}-copy-${Date.now().toString(36)}` },
      });
    }
    void apply(commands, "Pagina duplicata.");
    pageIndex = doc.pages.length;
  }

  function removePage() {
    if (!docPage || doc.pages.length <= 1) return;
    syncTextPage();
    if (!confirm(`Eliminare la pagina «${docPage.title}»?`)) return;
    void apply([{ op: "remove_page", id: docPage.id }], "Pagina eliminata.");
    pageIndex = Math.max(0, pageIndex - 1);
  }

  function movePage(delta: number) {
    if (!docPage) return;
    syncTextPage();
    const target = pageIndex + delta;
    if (target < 0 || target >= doc.pages.length) return;
    void apply([{ op: "move_page", id: docPage.id, index: target }], "Pagina spostata.");
    pageIndex = target;
  }

  function applyStandardFormat(value: string) {
    if (value === "custom") return;
    const [size, orientation] = value.split("|");
    void apply(
      [
        {
          op: "update_document",
          patch: {
            page: {
              ...doc.page,
              size,
              orientation,
              width_mm: null,
              height_mm: null,
            },
          },
        },
      ],
      "Formato foglio aggiornato."
    );
  }

  function applyCustomSize(width: number, height: number) {
    const w = Math.min(2000, Math.max(50, Math.round(width)));
    const h = Math.min(2000, Math.max(50, Math.round(height)));
    void apply(
      [
        {
          op: "update_document",
          patch: {
            page: {
              ...doc.page,
              size: "custom",
              orientation: "portrait",
              width_mm: w,
              height_mm: h,
            },
          },
        },
      ],
      `Formato personalizzato ${w}×${h} mm.`
    );
  }

  function setPageLayout(layout: string) {
    if (!docPage) return;
    void apply([{ op: "convert_layout", page: docPage.id, to: layout }], "Layout di pagina aggiornato.");
  }

  function renamePage(title: string) {
    if (!docPage) return;
    void apply([{ op: "update_page", id: docPage.id, patch: { title } }], "Pagina rinominata.");
  }

  function setHome() {
    if (!docPage) return;
    const commands: UniversalCommand[] = doc.pages.map((item) => ({
      op: "update_page",
      id: item.id,
      patch: { is_home: item.id === docPage.id },
    }));
    void apply(commands, "Pagina iniziale aggiornata.");
  }

  // ---------------------------------------------------------------- player
  function goToPage(target: string) {
    const index = doc.pages.findIndex((item) => item.id === target);
    if (index < 0) return;
    pageHistory = [...pageHistory, pageIndex];
    pageIndex = index;
    toggled = [];
    feedback = null;
  }

  function playAction(element: RenderedElement | { action?: any; spoken_text?: string; text?: string; id?: string }) {
    const action = element.action?.type ?? "append";
    if (action === "toggle") {
      const id = element.id ?? "";
      if (!id) return;
      const source = docPage?.elements.find((item) => item.id === id);
      const metadata = (source?.metadata ?? {}) as Record<string, unknown>;
      const group = typeof metadata.group === "string" ? metadata.group : null;
      if (group) {
        const groupIds = new Set(
          (docPage?.elements ?? [])
            .filter((item) => (item.metadata as Record<string, unknown> | undefined)?.group === group)
            .map((item) => item.id)
        );
        toggled = toggled.filter((item) => !groupIds.has(item));
      }
      toggled = toggled.filter((item) => item !== id);
      toggled = [...toggled, id];
      if (typeof metadata.correct === "boolean") {
        const message =
          typeof metadata.feedback === "string" ? metadata.feedback : element.spoken_text || element.text || "";
        feedback = {
          ok: metadata.correct,
          text: metadata.correct ? `Corretto: ${message}` : `Riprova: ${message}`,
        };
      }
      const spoken = element.spoken_text || "";
      if (spoken) onSpeak(spoken);
      return;
    }
    if (action === "navigate" && element.action?.target_page) {
      goToPage(element.action.target_page);
      return;
    }
    if (action === "back") {
      const previous = pageHistory[pageHistory.length - 1];
      if (previous !== undefined) {
        pageHistory = pageHistory.slice(0, -1);
        pageIndex = previous;
      }
      return;
    }
    if (action === "home") {
      const home = doc.pages.findIndex((item) => item.is_home);
      if (home >= 0) {
        pageHistory = [...pageHistory, pageIndex];
        pageIndex = home;
      }
      return;
    }
    if (action === "clear") {
      message = [];
      return;
    }
    const text = element.spoken_text || element.text || "";
    if (action === "speak") {
      onSpeak(text);
      return;
    }
    message = [...message, { id: `${Date.now()}-${message.length}`, text, svg: (element as RenderedElement).svg ?? null }];
  }

  function playCell(cell: { action?: any; spoken_text?: string; text?: string; svg?: string | null }, event: Event) {
    event.stopPropagation();
    playAction(cell as any);
  }

  function speakMessage() {
    const text = message.map((unit) => unit.text).filter(Boolean).join(" ");
    if (text) onSpeak(text);
  }

  // ------------------------------------------------------------- interazione
  let drag:
    | {
        mode: "move" | "resize";
        id: string;
        startX: number;
        startY: number;
        originalX: number;
        originalY: number;
        originalW: number;
        originalH: number;
      }
    | null = null;
  let dragPreview = $state<{ id: string; x: number; y: number; w: number; h: number } | null>(null);

  function pxToMm(value: number): number {
    return value / zoom / 3.779527559;
  }

  function snapTo(value: number, candidates: number[], threshold = 2): [number, number | null] {
    let best = value;
    let guide: number | null = null;
    for (const candidate of candidates) {
      if (!Number.isFinite(candidate)) continue;
      const distance = Math.abs(value - candidate);
      if (distance <= threshold && (guide === null || distance < Math.abs(best - guide))) {
        best = candidate;
        guide = candidate;
      }
    }
    return [best, guide];
  }

  function handleWheel(event: WheelEvent) {
    if (!(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    setZoomTo(zoom - Math.sign(event.deltaY) * 0.05);
  }

  let pinchStartDistance = 0;
  let pinchStartZoom = 1;

  function touchDistance(touches: TouchList): number {
    if (touches.length < 2) return 0;
    const dx = touches[0].clientX - touches[1].clientX;
    const dy = touches[0].clientY - touches[1].clientY;
    return Math.hypot(dx, dy);
  }

  function handleTouchStart(event: TouchEvent) {
    if (event.touches.length !== 2) return;
    pinchStartDistance = touchDistance(event.touches);
    pinchStartZoom = zoom;
  }

  function handleTouchMove(event: TouchEvent) {
    if (event.touches.length !== 2 || !pinchStartDistance) return;
    event.preventDefault();
    const scale = touchDistance(event.touches) / pinchStartDistance;
    setZoomTo(pinchStartZoom * scale);
  }

  function handleTouchEnd() {
    pinchStartDistance = 0;
  }

  function startRuler(event: PointerEvent, side: "left" | "right" | "top" | "bottom") {
    if (mode !== "edit" || readOnly) return;
    event.preventDefault();
    event.stopPropagation();
    rulerSide = side;
    rulerStart = side === "left" || side === "right" ? event.clientX : event.clientY;
    rulerStartMargin = doc.page.margin_mm;
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  }

  function moveRuler(event: PointerEvent) {
    if (!rulerSide || !page) return;
    const delta =
      rulerSide === "left" || rulerSide === "right"
        ? pxToMm(event.clientX - rulerStart)
        : pxToMm(event.clientY - rulerStart);
    const limit = (rulerSide === "left" || rulerSide === "right" ? page.width_mm : page.height_mm) / 2 - 5;
    const value =
      rulerSide === "left" || rulerSide === "top"
        ? rulerStartMargin + delta
        : rulerStartMargin - delta;
    marginPreview = Math.max(0, Math.min(limit, value));
  }

  function endRuler() {
    if (!rulerSide) return;
    const value = Math.round((marginPreview ?? doc.page.margin_mm) * 2) / 2;
    rulerSide = null;
    marginPreview = null;
    void apply(
      [{ op: "update_document", patch: { page: { margin_mm: value } } }],
      "Margini aggiornati."
    );
  }

  function startDrag(event: PointerEvent, element: RenderedElement, resize = false) {
    if (mode !== "edit" || element.locked || readOnly) return;
    const target = event.target as HTMLElement | null;
    if (!resize && target?.closest('[contenteditable="true"]')) return;
    event.preventDefault();
    event.stopPropagation();
    selectedId = element.id;
    selectedCellId = null;
    drag = {
      mode: resize ? "resize" : "move",
      id: element.id,
      startX: event.clientX,
      startY: event.clientY,
      originalX: element.x_mm,
      originalY: element.y_mm,
      originalW: element.w_mm,
      originalH: element.h_mm,
    };
    dragPreview = {
      id: element.id,
      x: element.x_mm,
      y: element.y_mm,
      w: element.w_mm,
      h: element.h_mm,
    };
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  }

  function moveDrag(event: PointerEvent) {
    if (!drag) return;
    const current = drag;
    const dx = pxToMm(event.clientX - current.startX);
    const dy = pxToMm(event.clientY - current.startY);
    if (current.mode === "move") {
      let x = Math.max(0, drag.originalX + dx);
      let y = Math.max(0, drag.originalY + dy);
      let guideX: number | null = null;
      let guideY: number | null = null;
      if (docPage?.layout !== "grid" && page) {
        const margin = doc.page.margin_mm;
        const others = page.elements.filter((item) => item.id !== current.id);
        const xs = [
          margin,
          page.width_mm - margin - current.originalW,
          (page.width_mm - current.originalW) / 2,
          ...others.flatMap((item) => [item.x_mm, item.x_mm + item.w_mm - current.originalW]),
        ];
        const ys = [
          margin,
          page.height_mm - margin - current.originalH,
          (page.height_mm - current.originalH) / 2,
          ...others.flatMap((item) => [item.y_mm, item.y_mm + item.h_mm - current.originalH]),
        ];
        [x, guideX] = snapTo(x, xs);
        [y, guideY] = snapTo(y, ys);
      }
      snapGuides = { x: guideX, y: guideY };
      dragPreview = {
        id: current.id,
        x,
        y,
        w: current.originalW,
        h: current.originalH,
      };
    } else {
      dragPreview = {
        id: current.id,
        x: current.originalX,
        y: current.originalY,
        w: Math.max(5, current.originalW + dx),
        h: Math.max(5, current.originalH + dy),
      };
    }
  }

  function endDrag() {
    if (!drag || !dragPreview || !docPage) {
      drag = null;
      dragPreview = null;
      return;
    }
    const preview = dragPreview;
    const target = drag;
    drag = null;
    dragPreview = null;
    snapGuides = { x: null, y: null };
    if (docPage.layout === "grid") {
      const page = rendered.pages[pageIndex];
      const margin = doc.page.margin_mm;
      const cellW = (page.width_mm - 2 * margin - docPage.grid.gap_mm * (docPage.grid.columns - 1)) / docPage.grid.columns;
      const cellH = (page.height_mm - 2 * margin - docPage.grid.gap_mm * (docPage.grid.rows - 1)) / docPage.grid.rows;
      const column = Math.max(0, Math.min(docPage.grid.columns - 1, Math.round((preview.x - margin) / (cellW + docPage.grid.gap_mm))));
      const row = Math.max(0, Math.min(docPage.grid.rows - 1, Math.round((preview.y - margin) / (cellH + docPage.grid.gap_mm))));
      updateElement(target.id, { row, column });
    } else if (target.mode === "move") {
      updateElement(target.id, { x_mm: preview.x, y_mm: preview.y });
    } else {
      void apply(
        [{
          op: "set_size",
          page: docPage.id,
          id: target.id,
          w_mm: preview.w,
          h_mm: preview.h,
        }],
        "Dimensioni aggiornate."
      );
    }
  }

  function anchorValue(z: number): "inline" | "front" | "behind" {
    if (z < 0) return "behind";
    if (z > 0) return "front";
    return "inline";
  }

  function setAnchor(kind: string) {
    if (!selectedElement) return;
    const z = kind === "behind" ? -1000 : kind === "front" ? 1000 : 0;
    updateElement(selectedElement.id, { z });
  }

  async function detachSelectedCell() {
    if (!selectedElement?.grid || !selectedCell || !selectedRendered || !docPage) return;
    const cell = selectedCell;
    const geometry = selectedRendered.cells.find((item) => item.id === cell.id);
    if (!geometry) return;
    const freeId = `free-${Date.now().toString(36)}`;
    const free = {
      id: freeId,
      kind: cell.symbol_id ? "symbol" : "text",
      text: cell.text,
      spoken_text: cell.spoken_text,
      concept_id: cell.concept_id,
      symbol_id: cell.symbol_id,
      svg: cell.svg,
      x_mm: selectedRendered.x_mm + geometry.x_mm,
      y_mm: selectedRendered.y_mm + geometry.y_mm,
      w_mm: geometry.w_mm,
      h_mm: geometry.h_mm,
      rotation: 0,
      z: selectedElement.z + 1,
      visible: true,
      locked: false,
      opacity: 1,
      style: cell.style,
      action: cell.action,
      link_page: null,
      group_id: null,
      children: [],
      grid: null,
    };
    const remaining = selectedElement.grid.cells.filter((item) => item.id !== cell.id);
    const commands: UniversalCommand[] = [];
    if (docPage.layout === "grid") {
      commands.push({ op: "convert_layout", page: docPage.id, to: "absolute" });
    }
    commands.push({ op: "add_element", page: docPage.id, element: free });
    if (remaining.length > 0) {
      commands.push({
        op: "update_grid",
        page: docPage.id,
        id: selectedElement.id,
        cells: remaining,
      });
    } else {
      commands.push({ op: "remove_element", page: docPage.id, id: selectedElement.id });
    }
    await apply(commands, "Cella svincolata: ora puoi trascinarla liberamente.");
    selectedId = freeId;
    selectedCellId = null;
  }

  function elementStyle(element: RenderedElement): string {
    const previewX = dragPreview?.id === element.id ? dragPreview.x : element.x_mm;
    const previewY = dragPreview?.id === element.id ? dragPreview.y : element.y_mm;
    const previewW = dragPreview?.id === element.id ? dragPreview.w : element.w_mm;
    const previewH = dragPreview?.id === element.id ? dragPreview.h : element.h_mm;
    const background = element.style.background || "transparent";
    const border =
      element.style.border_width && element.style.border_width > 0
        ? `${element.style.border_width}px ${element.style.border_style || "solid"} ${element.style.border_color || "#9aa7bb"}`
        : "none";
    return [
      `left:${previewX}mm`,
      `top:${previewY}mm`,
      `width:${Math.max(0, previewW)}mm`,
      `height:${Math.max(0, previewH)}mm`,
      `transform:rotate(${element.rotation}deg)`,
      `opacity:${element.opacity}`,
      `z-index:${element.z}`,
      `background:${background}`,
      `--uel-fill:${background}`,
      `--uel-stroke:${element.style.border_color || "#172033"}`,
      `border:${border}`,
      `border-radius:${element.style.border_radius ?? 2}mm`,
      `color:${element.style.color || "#111"}`,
      `font-size:${element.style.font_size ?? 4.2}mm`,
      `text-align:${element.style.align || "center"}`,
      `padding:${element.style.padding_mm ?? 1}mm`,
      // Dentro un elemento il pittogramma non può usare i 56px del foglio:
      // segue la dimensione del testo dell'elemento (font_size è in mm).
      `--symbol-size:${((element.style.font_size ?? 4.2) * 2.4).toFixed(2)}mm`,
      `--chip-radius:${Math.min(8, element.style.border_radius ?? 3)}px`,
    ].join(";");
  }

  function select(element: RenderedElement, event?: MouseEvent) {
    if (mode === "play" || readOnly) {
      playAction(element);
      return;
    }
    if (event?.shiftKey && selectedId && selectedId !== element.id) {
      extraSelected = extraSelected.includes(element.id)
        ? extraSelected.filter((id) => id !== element.id)
        : [...extraSelected, element.id];
      return;
    }
    selectedId = element.id;
    selectedCellId = null;
    extraSelected = [];
  }

  /// Clic sul pittogramma dell'elemento: apre le alternative nel pannello
  /// laterale senza toccare il testo.
  function requestSymbol(element: RenderedElement, event: Event) {
    if (mode !== "edit") return;
    event.stopPropagation();
    selectedId = element.id;
    selectedCellId = null;
    onSymbolRequest?.(element.id, null);
  }

  function groupSelection() {
    if (!docPage || !selectedId) return;
    const ids = [selectedId, ...extraSelected];
    if (ids.length < 2) {
      onToast("Seleziona almeno due elementi con Shift.", "warn");
      return;
    }
    const groupId = `group-${Date.now().toString(36)}`;
    void apply(
      [{ op: "group_elements", page: docPage.id, ids, group_id: groupId }],
      "Elementi raggruppati."
    );
    extraSelected = [];
  }

  function ungroupSelection() {
    if (!docPage || !selectedElement?.group_id) {
      onToast("L'elemento selezionato non è in un gruppo.", "warn");
      return;
    }
    void apply(
      [{ op: "ungroup_elements", page: docPage.id, group_id: selectedElement.group_id }],
      "Gruppo separato."
    );
    extraSelected = [];
  }

  function handleKey(event: KeyboardEvent) {
    if (mode !== "edit") return;
    if (event.key === "Escape") {
      selectedId = null;
      selectedCellId = null;
      extraSelected = [];
    }
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "z") {
      event.preventDefault();
      if (event.shiftKey) redo();
      else undo();
    }
    if ((event.key === "Backspace" || event.key === "Delete") && selectedId) {
      const target = event.target as HTMLElement;
      if (target.closest("input, textarea, select")) return;
      event.preventDefault();
      void apply(
        [{ op: "remove_element", page: docPage.id, id: selectedId }],
        "Elemento eliminato."
      );
      selectedId = null;
    }
  }

  onMount(() => {
    if (isTauri()) {
      void api.documentRender(doc).then((value) => { if (value) rendered = value; }).catch(() => {});
    }
  });

  async function printDocument() {
    syncTextPage();
    try {
      if (!isTauri()) {
        onToast("La stampa richiede l'app desktop.", "warn");
        return;
      }
      // PDF con il renderer reale (formato e orientamento corretti), aperto
      // nel visualizzatore di sistema: da lì si stampa con Cmd+P / Ctrl+P.
      const path = await api.printUniversal(doc);
      onToast(`PDF di stampa aperto: ${path}`, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function save() {
    syncTextPage();
    saving = true;
    void onSave(doc).finally(() => {
      saving = false;
      dirty = false;
      onDirtyChange?.(false);
    });
  }

  export function undoNow() {
    undo();
  }

  export function redoNow() {
    redo();
  }

  export function playNow() {
    syncTextPage();
    mode = mode === "play" ? "edit" : "play";
  }

  export function addPageNow() {
    addPage();
  }

  export function addTextPageNow() {
    void addTextPage();
  }

  export function togglePagesNow() {
    pagesOpen = !pagesOpen;
  }

  export function toggleRulerNow() {
    showRuler = !showRuler;
  }

  export function setDocumentTitle(title: string) {
    onTitleChange?.(title);
    void apply([{ op: "update_document", patch: { title } }], "");
  }

  export function setPageSettings(patch: Record<string, unknown>) {
    void apply(
      [{ op: "update_document", patch: { page: { ...doc.page, ...patch } } }],
      ""
    );
  }

  export function addElementNow(kind: string) {
    addElement(kind);
  }

  /// Ponte tra il pannello laterale e l'elemento selezionato. Il pannello
  /// possiede solo l'interfaccia: ogni modifica passa dagli stessi comandi
  /// dell'editor, quindi finisce nella cronologia Annulla/Ripeti.
  export function inspectorSetText(value: string, spoken = false) {
    setText(value, spoken);
  }

  export function inspectorSetNumber(field: string, value: number) {
    setNumber(field, value);
  }

  export function inspectorSetStyle(patch: Partial<UniversalStyle>) {
    setStyle(patch);
  }

  export function inspectorUpdate(patch: Record<string, unknown>) {
    if (selectedId) updateElement(selectedId, patch);
  }

  export function inspectorAnchor(kind: string) {
    setAnchor(kind);
  }

  export function inspectorDetachCell() {
    void detachSelectedCell();
  }

  export function inspectorApplySymbol(alternative: Alternative) {
    applySymbol(alternative);
  }

  /// Testo della cella selezionata (griglia): il pannello usa gli stessi
  /// comandi dell'editor, così la modifica entra nella cronologia.
  export function inspectorSetCellText(value: string) {
    if (selectedCell) setCellText(selectedCell.id, value);
  }

  export function inspectorSetCellSpoken(value: string) {
    if (!selectedElement?.grid || !selectedCell) return;
    const cells = selectedElement.grid.cells.map((cell) =>
      cell.id === selectedCell.id ? { ...cell, spoken_text: value } : cell
    );
    updateElement(selectedElement.id, {
      grid: { ...selectedElement.grid, cells },
    });
  }

  /// Aspetto della singola cella (testo, sfondo, bordo, spaziatura). La cella
  /// conserva il proprio stile: non cambia le altre parole della griglia.
  export function inspectorCellStyle(patch: Partial<UniversalStyle>) {
    if (!selectedElement?.grid || !selectedCell) return;
    const cells = selectedElement.grid.cells.map((cell) =>
      cell.id === selectedCell.id
        ? { ...cell, style: { ...(cell.style ?? {}), ...patch } }
        : cell
    );
    updateElement(selectedElement.id, {
      grid: { ...selectedElement.grid, cells },
    });
  }

  export function inspectorSetAction(action: { type: string; target_page: string | null } | null) {
    setAction(action);
  }

  export function inspectorSetDisposition(value: string) {
    if (!selectedId) return;
    updateElement(selectedId, { disposition: value });
  }

  export function inspectorEditCellCaA(cellId?: string) {
    const target = cellId ?? selectedCellId;
    if (target) editCellCaA(target);
  }

  export function inspectorSelectElement(id: string) {
    if (!docPage?.elements.some((element) => element.id === id)) return;
    selectedId = id;
    selectedCellId = null;
    extraSelected = [];
  }

  /// Duplica l'elemento selezionato, incluse celle, simboli, stile e azione.
  /// Il nuovo elemento si sposta di 3 mm per restare visibile e distinguibile.
  export async function inspectorDuplicate() {
    if (!docPage || !selectedElement) return;
    const source = clone(selectedElement);
    const id = `el-${Date.now().toString(36)}-${Math.floor(Math.random() * 1000)}`;
    const index = docPage.elements.findIndex((element) => element.id === source.id) + 1;
    const duplicate = {
      ...source,
      id,
      x_mm: Math.min(Math.max(0, (doc.page.width_mm ?? 210) - source.w_mm), source.x_mm + 3),
      y_mm: Math.min(Math.max(0, (doc.page.height_mm ?? 297) - source.h_mm), source.y_mm + 3),
      z: docPage.elements.length,
    };
    await apply(
      [{ op: "add_element", page: docPage.id, element: duplicate, index }],
      "Elemento duplicato."
    );
    selectedId = id;
    selectedCellId = null;
  }

  /// Compone una frase CAA dentro l'elemento selezionato: ogni parola diventa
  /// una cella con pittogramma e testo, come nel rigo normale. La forma, il
  /// fumetto o la casella mantengono sfondo e bordo; la frase resta composta
  /// da celle modificabili (testo, simbolo, righe e colonne).
  export async function inspectorComposeSentence(sentence: string) {
    const target = selectedElement;
    const pageNow = docPage;
    const text = sentence.trim();
    if (!target || !pageNow) return;
    if (!text) {
      onToast("Scrivi una frase prima di generare i pittogrammi.", "warn");
      return;
    }
    if (!isTauri()) {
      onToast("La simbolizzazione della frase richiede l'app desktop.", "warn");
      return;
    }
    const options = {
      intensity: caa?.intensity ?? "media",
      // La frase dentro una forma serve proprio a mostrare i pittogrammi:
      // usa sempre la modalità «sopra», anche se il documento è in solo testo.
      mode: "sopra" as const,
      document_key: caa?.documentKey ?? doc.id,
      commercial_only: caa?.commercialOnly ?? false,
      pack_id: caa?.packId ?? null,
      paragraph_id: `phrase-${target.id}`,
      symbolize_articles: caa?.symbolizeArticles ?? true,
      symbolize_prepositions: caa?.symbolizePrepositions ?? true,
      symbolize_conjunctions: caa?.symbolizeConjunctions ?? false,
      manual_units: [],
      split_ranges: [],
      profile_id: caa?.profileId ?? null,
    };
    const result = await api.symbolizeParagraph(text, options);
    const available = (page?.width_mm ?? 210) - 2 * (doc.page.margin_mm ?? 12);
    const desiredW = Math.max(40, Math.min(available, target.w_mm || 80));
    const columns = Math.max(1, Math.min(8, Math.floor(desiredW / 17)));
    const grid = buildPhraseGrid(result.tokens, { columns, gapMm: 1.2 });
    if (grid.cells.length === 0) {
      onToast("Nessuna parola da simbolizzare nella frase.", "warn");
      return;
    }
    const rowHeight = 20;
    const height = Math.max(target.h_mm, grid.rows * rowHeight + 3);
    const maxY = Math.max(0, (page?.height_mm ?? 297) - (doc.page.margin_mm ?? 12) - height);
    await apply(
      [{
        op: "update_element",
        page: pageNow.id,
        id: target.id,
        patch: {
          text,
          spoken_text: text,
          grid,
          w_mm: desiredW,
          h_mm: Math.min((page?.height_mm ?? 297) - 2 * (doc.page.margin_mm ?? 12), height),
          y_mm: Math.min(target.y_mm, maxY),
        },
      }],
      `Frase CAA: ${grid.cells.length} parole con pittogrammi.`
    );
    selectedCellId = null;
  }

  export function inspectorGridResize(deltaRows: number, deltaColumns: number) {
    resizeGrid(deltaRows, deltaColumns);
  }

  /// Layout della pagina corrente: libero, flusso o griglia. Serve al pannello
  /// laterale per rendere modificabile ogni modello senza la colonna interna.
  export function inspectorSetPageLayout(layout: string) {
    if (!docPage) return;
    void apply(
      [{ op: "convert_layout", page: docPage.id, to: layout }],
      "Layout della pagina aggiornato."
    );
  }

  export function inspectorSetPageGrid(columns: number, rows: number) {
    if (!docPage) return;
    const nextColumns = Math.max(1, Math.min(12, Math.round(columns)));
    const nextRows = Math.max(1, Math.min(60, Math.round(rows)));
    void apply(
      [{
        op: "set_page_grid",
        page: docPage.id,
        columns: nextColumns,
        rows: nextRows,
      }],
      `Griglia pagina: ${nextColumns}×${nextRows}.`
    );
  }

  export function inspectorClear() {
    selectedId = null;
    selectedCellId = null;
    extraSelected = [];
  }

  export function inspectorDelete() {
    if (!docPage || !selectedId) return;
    void apply(
      [{ op: "remove_element", page: docPage.id, id: selectedId }],
      "Elemento eliminato."
    );
    inspectorClear();
  }

  export function inspectorNudge(dx: number, dy: number) {
    if (!selectedElement) return;
    updateElement(selectedElement.id, {
      x_mm: Math.max(0, selectedElement.x_mm + dx),
      y_mm: Math.max(0, selectedElement.y_mm + dy),
    });
  }

  export function addGridNow() {
    addGrid();
  }

  export function groupNow() {
    groupSelection();
  }

  export function ungroupNow() {
    ungroupSelection();
  }

  export function zoomBy(delta: number) {
    zoom = Math.min(1.3, Math.max(0.45, Math.round((zoom + delta) * 100) / 100));
  }

  export function setZoomTo(value: number) {
    zoom = Math.min(1.3, Math.max(0.45, Math.round(value * 100) / 100));
  }

  export function zoomIn() {
    zoom = Math.min(1.3, Math.round((zoom + 0.1) * 100) / 100);
  }

  export function zoomOut() {
    zoom = Math.max(0.45, Math.round((zoom - 0.1) * 100) / 100);
  }

  export function zoomReset() {
    zoom = 0.78;
  }

  export function zoomFit() {
    zoom = 0.6;
  }

  export function saveNow() {
    syncTextPage();
    save();
  }

  export function printNow() {
    syncTextPage();
    void printDocument();
  }
</script>

{#snippet elementSymbol(element: RenderedElement)}
  <span
    class="uel__symbol"
    role={mode === "edit" ? "button" : "img"}
    tabindex={mode === "edit" ? 0 : -1}
    title={mode === "edit" ? "Alternative del pittogramma" : element.text}
    onclick={(event) => requestSymbol(element, event)}
    onkeydown={(event) => {
      if (mode === "edit" && (event.key === "Enter" || event.key === " ")) {
        event.preventDefault();
        requestSymbol(element, event);
      }
    }}
  >
    {#if element.image_asset}
      <img src={element.image_asset} alt="" />
    {:else if element.svg}
      {@html element.svg}
    {/if}
  </span>
{/snippet}

{#snippet inlineLabel(element: RenderedElement)}
  {@const stored = storedCaaText(element)}
  {#if stored && stored.tokens.length}
    <span
      class="uel__caa"
      role="text"
      title={mode === "edit" ? "Doppio clic per modificare testo e pittogrammi" : element.text}
      ondblclick={(event) => {
        event.stopPropagation();
        editSymbolOf(element);
      }}
    >
      {#each elementSegments(element.text, stored.tokens) as segment, index (index)}
        {#if segment.unit}
          <span
            class="caa-unit uel__caa-unit"
            data-caa-symbol={unitKey(segment.unit)}
            style={unitInlineStyle(segment.unit)}
            title={unitAccessibleTitle(segment.unit)}
            role="button"
            tabindex={mode === "edit" ? 0 : -1}
            onclick={(event) => {
              if (mode !== "edit") return;
              event.stopPropagation();
              editSymbolOf(element);
            }}
            onkeydown={(event) => {
              if (mode === "edit" && (event.key === "Enter" || event.key === " ")) {
                event.preventDefault();
                editSymbolOf(element);
              }
            }}
          >{segment.text}</span>
        {:else}{segment.text}{/if}
      {/each}
    </span>
  {:else}
    <span
      class="uel__label"
      contenteditable={mode === "edit" && !element.locked}
      spellcheck="false"
      title={mode === "edit" ? "Doppio clic per modificare con il motore CAA" : element.text}
      onfocus={() => (selectedId = element.id)}
      onpointerdown={(event) => event.stopPropagation()}
      ondblclick={(event) => {
        event.stopPropagation();
        editSymbolOf(element);
      }}
      onkeydown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          (event.currentTarget as HTMLElement).blur();
        }
        if (event.key === "Escape") {
          event.preventDefault();
          (event.currentTarget as HTMLElement).textContent = element.text;
          (event.currentTarget as HTMLElement).blur();
        }
      }}
      onblur={(event) => {
        const value = (event.currentTarget as HTMLElement).textContent ?? "";
        if (value !== element.text) setText(value);
      }}
    >{element.text || (element.kind === "text" || element.kind === "text_box" ? "" : element.kind)}</span>
  {/if}
{/snippet}

{#snippet elementLayer(elements: RenderedElement[])}
  <div class="ueditor__freelayer">
              {#each elements as element (element.id)}
                {#if element.visible}
                  <div
                    class="uel"
                    class:uel--selected={(selectedId === element.id || extraSelected.includes(element.id)) && mode === "edit"}
                    class:uel--locked={element.locked}
                    class:uel--symbol={element.kind === "symbol"}
                    class:uel--button={element.kind === "button"}
                    class:uel--speech={element.kind === "speech"}
                    class:uel--toggle={element.action?.type === "toggle"}
                    class:uel--on={toggled.includes(element.id)}
                    class:uel--behind={element.disposition === "behind"}
                    class:uel--front={element.disposition === "front"}
                    class:uel--wrap={element.disposition === "wrap_left" || element.disposition === "wrap_right"}
                    data-action={element.action?.type ?? ""}
                    style={elementStyle(element)}
                    role={mode === "play" ? "button" : "group"}
                    tabindex={mode === "play" ? 0 : -1}
                    onclick={(event) => select(element, event)}
                    onkeydown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        select(element);
                      }
                    }}
                    onpointerdown={(event) => startDrag(event, element)}
                    onpointermove={moveDrag}
                    onpointerup={endDrag}
                    onpointercancel={endDrag}
                  >
                    {#if element.cells?.length}
                      <span class="uel__grid">
                        {#each element.cells as cell (cell.id)}
                          {#if !cell.hidden}
                            {@const cellCaa = storedCaaText(cell)}
                            <button
                              class="uel__cell"
                              style={`left:${cell.x_mm}mm;top:${cell.y_mm}mm;width:${cell.w_mm}mm;height:${cell.h_mm}mm;background:${cell.style.background || "transparent"};border:${(cell.style.border_style === "none" ? 0 : cell.style.border_width) || 0}mm ${cell.style.border_style && cell.style.border_style !== "none" ? cell.style.border_style : "solid"} ${cell.style.border_color || "#9aa7bb"};border-radius:${cell.style.border_radius || 0}mm;padding:${cell.style.padding_mm || 0}mm;box-sizing:border-box;font-size:${cell.style.font_size || 3.2}mm;color:${cell.style.color || "#111"};`}
                              onpointerdown={(event) => event.stopPropagation()}
                              ondblclick={(event) => {
                                if (mode !== "edit") return;
                                event.stopPropagation();
                                selectedId = element.id;
                                editCellCaA(cell.id);
                              }}
                              onclick={(event) => {
                                if (mode === "play") playCell(cell, event);
                                else {
                                  event.stopPropagation();
                                  selectedId = element.id;
                                  selectedCellId = cell.id;
                                }
                              }}
                            >
                              {#if cellCaa && cellCaa.tokens.length}
                                <span
                                  class="uel__caa-cell"
                                  role="text"
                                  title={mode === "edit"
                                    ? "Doppio clic per modificare la frase e i pittogrammi"
                                    : cell.text}
                                >
                                  {#each elementSegments(cell.text, cellCaa.tokens) as segment, segmentIndex (segmentIndex)}
                                    {#if segment.unit}
                                      <span
                                        class="caa-unit uel__caa-unit"
                                        data-caa-symbol={unitKey(segment.unit)}
                                        style={unitInlineStyle(segment.unit)}
                                        title={unitAccessibleTitle(segment.unit)}
                                        role={mode === "edit" ? "button" : "text"}
                                        tabindex={mode === "edit" ? 0 : -1}
                                        onclick={(event) => {
                                          if (mode !== "edit") return;
                                          event.stopPropagation();
                                          selectedId = element.id;
                                          editCellCaA(cell.id);
                                        }}
                                        onkeydown={(event) => {
                                          if (
                                            mode === "edit" &&
                                            (event.key === "Enter" || event.key === " ")
                                          ) {
                                            event.preventDefault();
                                            selectedId = element.id;
                                            editCellCaA(cell.id);
                                          }
                                        }}
                                      >{segment.text}</span>
                                    {:else}{segment.text}{/if}
                                  {/each}
                                </span>
                              {:else}
                                {#if cell.image_asset}
                                  <span
                                    class="uel__cell-symbol"
                                    role={mode === "edit" ? "button" : "img"}
                                    tabindex={mode === "edit" ? 0 : -1}
                                    title={mode === "edit" ? `Alternative per «${cell.text || "cella"}»` : cell.text}
                                    onclick={(event) => {
                                      if (mode !== "edit") return;
                                      event.stopPropagation();
                                      selectedId = element.id;
                                      selectedCellId = cell.id;
                                      onSymbolRequest?.(element.id, cell.id);
                                    }}
                                    onkeydown={(event) => {
                                      if (mode === "edit" && (event.key === "Enter" || event.key === " ")) {
                                        event.preventDefault();
                                        selectedId = element.id;
                                        selectedCellId = cell.id;
                                        onSymbolRequest?.(element.id, cell.id);
                                      }
                                    }}
                                  ><img src={cell.image_asset} alt="" /></span>
                                {:else if cell.svg}
                                  <span
                                    class="uel__cell-symbol"
                                    role={mode === "edit" ? "button" : "img"}
                                    tabindex={mode === "edit" ? 0 : -1}
                                    title={mode === "edit" ? `Alternative per «${cell.text || "cella"}»` : cell.text}
                                    onclick={(event) => {
                                      if (mode !== "edit") return;
                                      event.stopPropagation();
                                      selectedId = element.id;
                                      selectedCellId = cell.id;
                                      onSymbolRequest?.(element.id, cell.id);
                                    }}
                                    onkeydown={(event) => {
                                      if (mode === "edit" && (event.key === "Enter" || event.key === " ")) {
                                        event.preventDefault();
                                        selectedId = element.id;
                                        selectedCellId = cell.id;
                                        onSymbolRequest?.(element.id, cell.id);
                                      }
                                    }}
                                  >{@html cell.svg}</span>
                                {/if}
                                <span
                                  contenteditable={mode === "edit"}
                                  spellcheck="false"
                                  onpointerdown={(event) => event.stopPropagation()}
                                  onkeydown={(event) => {
                                    if (event.key === "Enter") {
                                      event.preventDefault();
                                      (event.currentTarget as HTMLElement).blur();
                                    }
                                  }}
                                  onblur={(event) => {
                                    const value = (event.currentTarget as HTMLElement).textContent ?? "";
                                    if (value !== cell.text) setCellText(cell.id, value);
                                  }}
                                >{cell.text}</span>
                              {/if}
                            </button>
                          {/if}
                        {/each}
                      </span>
                    {:else if element.kind === "button" || element.kind === "shape"}
                      {@render elementSymbol(element)}
                      {@render inlineLabel(element)}
                    {:else if element.kind === "token_slot"}
                      <span class="uel__token" aria-hidden="true"></span>
                    {:else if element.image_asset}
                      {@render elementSymbol(element)}
                      {@render inlineLabel(element)}
                    {:else if element.svg}
                      {@render elementSymbol(element)}
                      {@render inlineLabel(element)}
                    {:else}
                      {@render inlineLabel(element)}
                    {/if}
                    {#if element.locked}<span class="uel__lock">🔒</span>{/if}
                    {#if mode === "edit" && selectedId === element.id && !element.locked}
                      <span
                        class="uel__handle"
                        role="presentation"
                        onpointerdown={(event) => startDrag(event, element, true)}
                        onpointermove={moveDrag}
                        onpointerup={endDrag}
                      ></span>
                    {/if}
                  </div>
                {/if}
              {/each}
  </div>
{/snippet}
<svelte:window on:keydown={handleKey} />

<div class="ueditor" class:ueditor--play={mode === "play"} data-profile-size={profile?.accessibility?.large_symbols ? "large" : "normal"}>
  {#if !embedded}
  <header class="ueditor__bar">
    <button class="btn" onclick={onClose}>← Home</button>
    <input class="ueditor__title" value={doc.title} aria-label="Titolo del documento" onchange={(event) => { onTitleChange?.(event.currentTarget.value); void apply([{ op: "update_document", patch: { title: event.currentTarget.value } }], "Titolo aggiornato."); }} readonly={mode === "play"} />
    <span class="ueditor__badge">{doc.kind}</span>
    <span class="ueditor__badge">{doc.pages.length} pagine</span>
    <span class="grow"></span>
    {#if mode === "edit"}
      <button class="btn" disabled={busy || history.length === 0} onclick={undo}>Annulla</button>
      <button class="btn" disabled={busy || redoStack.length === 0} onclick={redo}>Ripeti</button>
      <button class="btn" onclick={printDocument}>Stampa</button>
      <button class="btn" onclick={() => (mode = "play")}>Anteprima</button>
      {#if onOpenAi}
        <button class="btn" onclick={onOpenAi}>IA</button>
      {/if}
      {#if onOpenStudio}
        <button class="btn" onclick={onOpenStudio}>Genera</button>
      {/if}
      <button class="btn" onclick={save} disabled={saving}>{saving ? "Salvo…" : "Salva .caadoc"}</button>
      {#if onSavePlayer}
        <button class="btn" onclick={() => void onSavePlayer(doc)}>Salva .caaplay</button>
      {/if}
      {#if onExport}
        <button class="btn" onclick={() => void onExport(doc, "html")}>HTML</button>
        <button class="btn" onclick={() => void onExport(doc, "pdf")}>PDF</button>
        <button class="btn" onclick={() => void onExport(doc, "png")}>PNG</button>
        <button class="btn" onclick={() => void onExport(doc, "docx")}>DOCX</button>
        <button class="btn" onclick={() => void onExport(doc, "pptx")}>PPTX</button>
      {/if}
    {:else}
      <button class="btn" onclick={onOpen}>Apri</button>
      <button class="btn" onclick={onImport}>Importa</button>
      {#if canEdit}
        <button class="btn btn--solid" onclick={onEdit}>Modifica layout e contenuti</button>
      {/if}
      <button class="btn" onclick={printDocument}>Stampa</button>
      {#if onToggleFullscreen}
        <button class="btn" onclick={onToggleFullscreen}>Schermo intero</button>
      {/if}
      <button class="btn" onclick={() => (pageHistory = [...pageHistory, pageIndex], pageIndex = Math.max(0, doc.pages.findIndex((p) => p.is_home)))}>Home pagina</button>
    {/if}
  </header>
  {/if}

  {#if readOnly && canEdit}
    <div class="ueditor__preview">
      <span>Anteprima non modificabile.</span>
      <button class="btn btn--solid" onclick={onEdit}>Modifica layout e contenuti</button>
    </div>
  {/if}

  <div class="ueditor__body" class:ueditor__body--nopages={!pagesOpen} class:ueditor__body--noprops={embedded}>
    {#if !pagesOpen}
      <button
        class="ueditor__pages-reopen btn btn--ghost"
        onclick={() => (pagesOpen = true)}
        title="Mostra il pannello delle pagine"
      >Pagine »</button>
    {/if}
    {#if pagesOpen}
    <aside class="ueditor__pages" aria-label="Pagine">
      <div class="ueditor__pages-head">
        <strong>Pagine</strong>
        {#if mode === "edit"}
          <button class="mini" onclick={addPage} title="Aggiungi pagina">+</button>
          <button class="mini" onclick={() => void addTextPage()} title="Aggiungi pagina di testo CAA">T+</button>
        {/if}
        <button class="mini" onclick={() => (pagesOpen = false)} title="Riduci il pannello delle pagine">«</button>
      </div>
      {#each rendered.pages as item (item.id)}
        <button
          class="thumb"
          class:thumb--active={item.index === pageIndex}
          onclick={() => selectPage(item.index)}
          title={item.title}
        >
          <span
            class="thumb__paper"
            style={`aspect-ratio:${item.width_mm}/${item.height_mm};background:${item.background || "#ffffff"}`}
          >
            {#each item.elements as element (element.id)}
              {#if element.visible}
                <span
                  class="thumb__el"
                  style={`left:${thumbPct(element.x_mm, item.width_mm)}%;top:${thumbPct(element.y_mm, item.height_mm)}%;width:${thumbPct(element.w_mm, item.width_mm)}%;height:${thumbPct(element.h_mm, item.height_mm)}%;opacity:${element.opacity};${element.rotation ? `transform:rotate(${element.rotation}deg);` : ""}`}
                >
                  {#if element.image_asset}
                    <img src={element.image_asset} alt="" style="width:100%;height:100%;object-fit:contain" />
                  {:else if element.svg}
                    {@html element.svg}
                  {:else if element.cells?.length}
                    {#each element.cells as cell (cell.id)}
                      {#if !cell.hidden}
                        <span
                          class="thumb__cell"
                          style={`left:${thumbPct(cell.x_mm, element.w_mm)}%;top:${thumbPct(cell.y_mm, element.h_mm)}%;width:${thumbPct(cell.w_mm, element.w_mm)}%;height:${thumbPct(cell.h_mm, element.h_mm)}%;background:${cell.style.background || "transparent"}`}
                        >
                          {#if cell.image_asset}
                            <img src={cell.image_asset} alt="" style="width:100%;height:100%;object-fit:contain" />
                          {:else if cell.svg}{@html cell.svg}{/if}
                          <span class="thumb__cell-text">{cell.text}</span>
                        </span>
                      {/if}
                    {/each}
                  {:else if element.kind === "shape"}
                    <span
                      class="thumb__shape"
                      style={`background:${element.style.background || "transparent"};border-color:${element.style.border_color || "transparent"};border-width:${element.style.border_width ? `${Math.max(1, element.style.border_width)}px` : "0"};`}
                    ></span>
                  {:else if element.kind === "speech"}
                    <span
                      class="thumb__speech"
                      style={`background:${element.style.background || "#fff"};border-color:${element.style.border_color || "#172033"};border-radius:${element.style.border_radius ?? 4}px;`}
                    >{element.text}</span>
                  {:else if element.text}
                    <span class="thumb__text">{element.text}</span>
                  {/if}
                </span>
              {/if}
            {/each}
          </span>
          <span class="thumb__title">{item.title}{item.is_home ? " · home" : ""}</span>
        </button>
      {/each}
      {#if mode === "edit" && docPage}
        <div class="ueditor__page-tools">
          <button class="mini" onclick={duplicatePage}>Duplica</button>
          <button class="mini" onclick={() => movePage(-1)}>Su</button>
          <button class="mini" onclick={() => movePage(1)}>Giù</button>
          <button class="mini mini--danger" onclick={removePage}>Elimina</button>
          <button class="mini" onclick={setHome}>Home</button>
        </div>
        <input class="ueditor__page-title" value={docPage.title} aria-label="Titolo della pagina" onchange={(event) => renamePage(event.currentTarget.value)} />
        <label class="ueditor__field">
          <span>Layout pagina</span>
          <select
            value={docPage.layout}
            disabled={isTextPage(docPage.id)}
            onchange={(event) => setPageLayout(event.currentTarget.value)}
          >
            <option value="flow">Documento (flusso)</option>
            <option value="absolute">Presentazione (libero)</option>
            <option value="grid">Griglia</option>
          </select>
        </label>
        {#if docPage.layout === "grid"}
          <div class="ueditor__two">
            <label class="ueditor__field"><span>Colonne</span><input type="number" min="1" max="12" value={docPage.grid.columns} onchange={(event) => void apply([{ op: "set_page_grid", page: docPage.id, columns: Number(event.currentTarget.value), rows: docPage.grid.rows }], "Griglia aggiornata.")} /></label>
            <label class="ueditor__field"><span>Righe</span><input type="number" min="1" max="60" value={docPage.grid.rows} onchange={(event) => void apply([{ op: "set_page_grid", page: docPage.id, columns: docPage.grid.columns, rows: Number(event.currentTarget.value) }], "Griglia aggiornata.")} /></label>
          </div>
        {/if}
      {/if}
    </aside>
    {/if}

    <main
      class="ueditor__stage"
      class:ueditor__stage--play={mode === "play"}
      onwheel={handleWheel}
      ontouchstart={handleTouchStart}
      ontouchmove={handleTouchMove}
      ontouchend={handleTouchEnd}
      ontouchcancel={handleTouchEnd}
    >
      {#if mode === "edit" && !embedded}
        <div class="ueditor__tools">
          <button class="btn" onclick={() => addElement("text")}>Testo</button>
          <button class="btn" onclick={() => addElement("text_box")}>Casella</button>
          <button class="btn" onclick={() => addElement("button")}>Pulsante</button>
          <button class="btn" onclick={() => addElement("shape")}>Forma</button>
          <button class="btn" onclick={() => addElement("placeholder")}>Segnaposto</button>
          <button class="btn" onclick={() => (gridPanel = !gridPanel)}>Griglia</button>
          {#if gridPanel}
            <span class="ueditor__gridbox">
              <label>Righe <input type="number" min="1" max="40" bind:value={gridRows} /></label>
              <label>Colonne <input type="number" min="1" max="20" bind:value={gridColumns} /></label>
              <button class="btn btn--solid" onclick={addGrid}>Crea {gridColumns}×{gridRows} ({gridColumns * gridRows} celle)</button>
            </span>
          {/if}
          <button class="btn" onclick={groupSelection}>Raggruppa</button>
          <button class="btn" onclick={ungroupSelection}>Separa</button>
          {#if !embedded}
            <button class="btn" onclick={() => (pagesOpen = !pagesOpen)}>
              {pagesOpen ? "Nascondi pagine" : "Mostra pagine"}
            </button>
          {/if}
          {#if !embedded}
            <button class="btn" class:btn--solid={showRuler} onclick={() => (showRuler = !showRuler)}>
              Righello
            </button>
          {/if}
        </div>
      {/if}
      {#if mode === "edit"}
        <div class="ueditor__zoombar" role="group" aria-label="Zoom del documento">
          <button class="mini" onclick={() => zoomBy(-0.05)} title="Riduci zoom">−</button>
          <input
            type="range"
            min="0.45"
            max="1.3"
            step="0.05"
            bind:value={zoom}
            aria-label="Zoom"
          />
          <button class="mini" onclick={() => zoomBy(0.05)} title="Aumenta zoom">+</button>
          <span class="ueditor__zoomvalue">{Math.round(zoom * 100)}%</span>
        </div>
      {/if}
      {#if marginPreview !== null}
        <span class="ueditor__margin-badge">{marginPreview} mm</span>
      {/if}
      {#if mode === "edit" && docPage && !isTextPage(docPage.id)}
        <p class="ueditor__flowhint">
          Doppio clic in uno spazio libero per scrivere un nuovo paragrafo. Gli elementi
          selezionati si dispongono dal pannello Documento.
        </p>
      {/if}
      {#if docPage && isTextPage(docPage.id) && mode !== "play" && caa}
        <div class="ueditor__textpage">
          <div
            class="paper"
            data-uppercase={uppercase ? "true" : "false"}
            style={`zoom:${zoom};width:${page?.width_mm ?? 210}mm;min-height:${page?.height_mm ?? 297}mm;max-width:none;padding:${doc.page.margin_mm ?? 15}mm;`}
          >
          {#key docPage.id}
            <CaaEditor
              bind:this={textEditor}
              {...caa}
              onAnalyzed={(map) => {
                textTokens = map;
                caa?.onAnalyzed?.(map);
              }}
              onDocChanged={() => {
                dirty = true;
                onDirtyChange?.(true);
                caa?.onDocChanged?.();
                scheduleTextSync();
              }}
            />
          {/key}
          <!-- Sovrapposizione libera: fumetti, immagini e forme convivono con
               il testo della pagina CAA. Gli elementi di flusso `t-…` sono la
               copia tecnica per miniature/export e non vanno duplicati qui. -->
          {@render elementLayer(page.elements.filter((element) => !element.id.startsWith("t-")))}
          </div>
        </div>
      {:else if page}
        <div class="ueditor__pagewrap" style={`zoom:${zoom}`}>
          <div
            class="ueditor__page"
            style={`width:${page.width_mm}mm;height:${page.height_mm}mm;background:${page.background || "#fff"}`}
            role={mode === "play" ? "group" : "application"}
            aria-label={`${page.title} (${page.width_mm}×${page.height_mm} mm)`}
            ondblclick={pageDoubleClick}
          >
            {#if showRuler}
              <span class="ueditor__ruler ueditor__ruler--top" aria-hidden="true"></span>
              <span class="ueditor__ruler ueditor__ruler--left" aria-hidden="true"></span>
              <span
                class="ueditor__margin-line ueditor__margin-line--left"
                style={`left:${marginPreview ?? doc.page.margin_mm}mm`}
              ></span>
              <span
                class="ueditor__margin-line ueditor__margin-line--right"
                style={`right:${marginPreview ?? doc.page.margin_mm}mm`}
              ></span>
              <span
                class="ueditor__margin-line ueditor__margin-line--top"
                style={`top:${marginPreview ?? doc.page.margin_mm}mm`}
              ></span>
              <span
                class="ueditor__margin-line ueditor__margin-line--bottom"
                style={`bottom:${marginPreview ?? doc.page.margin_mm}mm`}
              ></span>
              <button
                class="ueditor__margin-handle ueditor__margin-handle--left"
                aria-label="Trascina il margine sinistro"
                style={`left:${marginPreview ?? doc.page.margin_mm}mm`}
                onpointerdown={(event) => startRuler(event, "left")}
                onpointermove={moveRuler}
                onpointerup={endRuler}
              ></button>
              <button
                class="ueditor__margin-handle ueditor__margin-handle--right"
                aria-label="Trascina il margine destro"
                style={`right:${marginPreview ?? doc.page.margin_mm}mm`}
                onpointerdown={(event) => startRuler(event, "right")}
                onpointermove={moveRuler}
                onpointerup={endRuler}
              ></button>
              <button
                class="ueditor__margin-handle ueditor__margin-handle--top"
                aria-label="Trascina il margine superiore"
                style={`top:${marginPreview ?? doc.page.margin_mm}mm`}
                onpointerdown={(event) => startRuler(event, "top")}
                onpointermove={moveRuler}
                onpointerup={endRuler}
              ></button>
              <button
                class="ueditor__margin-handle ueditor__margin-handle--bottom"
                aria-label="Trascina il margine inferiore"
                style={`bottom:${marginPreview ?? doc.page.margin_mm}mm`}
                onpointerdown={(event) => startRuler(event, "bottom")}
                onpointermove={moveRuler}
                onpointerup={endRuler}
              ></button>
            {/if}
            {#if snapGuides.x !== null}
              <span class="ueditor__snap ueditor__snap--v" style={`left:${snapGuides.x}mm`}></span>
            {/if}
            {#if snapGuides.y !== null}
              <span class="ueditor__snap ueditor__snap--h" style={`top:${snapGuides.y}mm`}></span>
            {/if}
            {@render elementLayer(page.elements)}
          </div>
        </div>
      {/if}
      {#if mode === "play"}
        <div class="ueditor__message">
          <div class="ueditor__chips">
            {#if message.length === 0}<span class="hint">Tocca i pittogrammi per comporre la frase.</span>{/if}
            {#each message as unit (unit.id)}
              <button class="ueditor__chip" onclick={() => (message = message.filter((item) => item.id !== unit.id))}>{unit.text} ×</button>
            {/each}
          </div>
          {#if feedback}
            <p class="ueditor__feedback" class:ueditor__feedback--ok={feedback.ok} role="status">
              {feedback.text}
            </p>
          {/if}
          <div class="ueditor__message-actions">
            <button class="btn" onclick={() => (message = [])} disabled={message.length === 0}>Svuota</button>
            <button class="btn btn--solid" onclick={speakMessage} disabled={message.length === 0}>Parla frase</button>
          </div>
        </div>
      {/if}
    </main>

    {#if mode === "edit" && !embedded}
      {#if selectedElement}
      <aside class="ueditor__properties" aria-label="Proprietà">
        {#if selectedElement && selectedRendered}
          <h3>Proprietà</h3>
          <p class="hint">{selectedElement.kind} · {selectedElement.id}{selectedElement.group_id ? ` · gruppo ${selectedElement.group_id}` : ""}</p>
          <label class="ueditor__field"><span>Testo</span><textarea value={selectedElement.text} onchange={(event) => setText(event.currentTarget.value)}></textarea></label>
          <label class="ueditor__field"><span>Testo pronunciato</span><input value={selectedElement.spoken_text} onchange={(event) => setText(event.currentTarget.value, true)} /></label>
          <div class="ueditor__two">
            <label class="ueditor__field"><span>X (mm)</span><input type="number" step="0.5" value={selectedRendered.x_mm} onchange={(event) => setNumber("x_mm", Number(event.currentTarget.value))} /></label>
            <label class="ueditor__field"><span>Y (mm)</span><input type="number" step="0.5" value={selectedRendered.y_mm} onchange={(event) => setNumber("y_mm", Number(event.currentTarget.value))} /></label>
            <label class="ueditor__field"><span>Larghezza</span><input type="number" step="0.5" value={selectedRendered.w_mm} onchange={(event) => setNumber("w_mm", Number(event.currentTarget.value))} /></label>
            <label class="ueditor__field"><span>Altezza</span><input type="number" step="0.5" value={selectedRendered.h_mm} onchange={(event) => setNumber("h_mm", Number(event.currentTarget.value))} /></label>
            <label class="ueditor__field"><span>Rotazione</span><input type="number" min="-360" max="360" step="1" value={selectedElement.rotation} onchange={(event) => setNumber("rotation", Number(event.currentTarget.value))} /></label>
            <label class="ueditor__field"><span>Livello</span><input type="number" value={selectedElement.z} onchange={(event) => updateElement(selectedElement.id, { z: Number(event.currentTarget.value) })} /></label>
            <label class="ueditor__field"><span>Ancoraggio</span>
              <select value={anchorValue(selectedElement.z)} onchange={(event) => setAnchor(event.currentTarget.value)}>
                <option value="inline">In linea con il testo</option>
                <option value="front">Davanti al testo</option>
                <option value="behind">Dietro al testo</option>
              </select>
            </label>
          </div>
          <p class="hint">L'ancoraggio cambia il livello di sovrapposizione: davanti/dietro non modificano il testo.</p>
          <div class="ueditor__two">
            <label class="ueditor__field"><span>Colore testo</span><input type="color" value={selectedElement.style.color || "#111111"} onchange={(event) => setStyle({ color: event.currentTarget.value })} /></label>
            <label class="ueditor__field"><span>Sfondo</span><input type="color" value={selectedElement.style.background || "#ffffff"} onchange={(event) => setStyle({ background: event.currentTarget.value })} /></label>
            <label class="ueditor__field"><span>Bordo</span><input type="color" value={selectedElement.style.border_color || "#9aa7bb"} onchange={(event) => setStyle({ border_color: event.currentTarget.value })} /></label>
            <label class="ueditor__field"><span>Spessore</span><input type="number" min="0" max="8" step="0.2" value={selectedElement.style.border_width ?? 0} onchange={(event) => setStyle({ border_width: Number(event.currentTarget.value) })} /></label>
          </div>
          <div class="ueditor__row">
            <label><input type="checkbox" checked={selectedElement.visible} onchange={(event) => updateElement(selectedElement.id, { visible: event.currentTarget.checked })} /> Visibile</label>
            <label><input type="checkbox" checked={selectedElement.locked} onchange={(event) => updateElement(selectedElement.id, { locked: event.currentTarget.checked })} /> Bloccato</label>
          </div>
          {#if selectedElement.grid && selectedCell}
            <button class="btn" onclick={() => void detachSelectedCell()}>
              Svincola cella e trascinala
            </button>
          {/if}
          <h4>Pittogramma</h4>
          <div class="ueditor__row">
            <input bind:value={searchQuery} placeholder="Cerca un simbolo…" onkeydown={(event) => { if (event.key === "Enter") { event.preventDefault(); void searchSymbol(); } }} />
            <button class="btn" onclick={() => void searchSymbol()} disabled={searchBusy}>Cerca</button>
          </div>
          {#if searchResults.length > 0}
            <div class="ueditor__results">
              {#each searchResults as alternative (alternative.symbol_id)}
                <button class="ueditor__result" onclick={() => applySymbol(alternative)} title={alternative.license}>
                  {#if alternative.image}
                    <span class="ueditor__result-media"><img src={alternative.image} alt="" /></span>
                  {:else if alternative.svg}<span class="ueditor__result-media">{@html alternative.svg}</span>{/if}
                  <span>{alternative.label}</span>
                </button>
              {/each}
            </div>
          {/if}
          {#if onlineBusy}
            <p class="hint">Cerco nelle API pubbliche…</p>
          {:else if onlineResults.length > 0}
            <p class="hint">{onlineNote}</p>
            <div class="ueditor__results">
              {#each onlineResults as online (online.provider + online.id)}
                <button class="ueditor__result" onclick={() => void useOnlineResult(online)} title={`${online.license} · ${online.source}`}>
                  {#if online.thumb}<img class="ueditor__result-thumb" src={online.thumb} alt="" />{/if}
                  <span>{online.title}</span>
                  <small>{online.source}</small>
                </button>
              {/each}
            </div>
          {:else if onlineNote}
            <p class="hint">{onlineNote}</p>
          {/if}
          <h4>Azione</h4>
          <label class="ueditor__field"><span>Tipo</span>
            <select value={selectedElement.action?.type ?? "append"} onchange={(event) => setAction({ type: event.currentTarget.value, target_page: selectedElement.action?.target_page })}>
              <option value="append">Aggiungi alla frase</option>
              <option value="speak">Pronuncia</option>
              <option value="navigate">Vai alla pagina</option>
              <option value="back">Indietro</option>
              <option value="home">Home</option>
              <option value="clear">Svuota frase</option>
            </select>
          </label>
          {#if selectedElement.action?.type === "navigate"}
            <label class="ueditor__field"><span>Pagina</span>
              <select value={selectedElement.action?.target_page ?? ""} onchange={(event) => setAction({ type: "navigate", target_page: event.currentTarget.value })}>
                <option value="">Scegli…</option>
                {#each doc.pages as item (item.id)}<option value={item.id}>{item.title}</option>{/each}
              </select>
            </label>
          {/if}
          {#if selectedCell}
            <h4>Cella selezionata</h4>
            <p class="hint">{selectedCell.id}</p>
            <label class="ueditor__field"><span>Testo cella</span><input value={selectedCell.text} onchange={(event) => {
              const cells = clone(selectedElement?.grid?.cells ?? []);
              const target = cells.find((cell) => cell.id === selectedCell.id);
              if (target && selectedElement) {
                target.text = event.currentTarget.value;
                updateElement(selectedElement.id, { grid: { ...selectedElement.grid, cells } });
              }
            }} /></label>
            <label class="ueditor__field"><span>Voce cella</span><input value={selectedCell.spoken_text} onchange={(event) => {
              const cells = clone(selectedElement?.grid?.cells ?? []);
              const target = cells.find((cell) => cell.id === selectedCell.id);
              if (target && selectedElement) {
                target.spoken_text = event.currentTarget.value;
                updateElement(selectedElement.id, { grid: { ...selectedElement.grid, cells } });
              }
            }} /></label>
            <label class="ueditor__field"><span>Azione cella</span>
              <select value={selectedCell.action?.type ?? "append"} onchange={(event) => {
                const cells = clone(selectedElement?.grid?.cells ?? []);
                const target = cells.find((cell) => cell.id === selectedCell.id);
                if (target && selectedElement) {
                  target.action = {
                    type: event.currentTarget.value,
                    target_page: event.currentTarget.value === "navigate" ? (target.action?.target_page ?? doc.pages[0]?.id ?? null) : null,
                  };
                  updateElement(selectedElement.id, { grid: { ...selectedElement.grid, cells } });
                }
              }}>
                <option value="append">Aggiungi alla frase</option>
                <option value="speak">Pronuncia</option>
                <option value="navigate">Vai alla pagina</option>
                <option value="back">Indietro</option>
                <option value="home">Home</option>
                <option value="clear">Svuota frase</option>
              </select>
            </label>
            {#if selectedCell.action?.type === "navigate"}
              <label class="ueditor__field"><span>Pagina cella</span>
                <select value={selectedCell.action?.target_page ?? ""} onchange={(event) => {
                  const cells = clone(selectedElement?.grid?.cells ?? []);
                  const target = cells.find((cell) => cell.id === selectedCell.id);
                  if (target && selectedElement) {
                    target.action = { type: "navigate", target_page: event.currentTarget.value };
                    updateElement(selectedElement.id, { grid: { ...selectedElement.grid, cells } });
                  }
                }}>
                  <option value="">Scegli…</option>
                  {#each doc.pages as item (item.id)}<option value={item.id}>{item.title}</option>{/each}
                </select>
              </label>
            {/if}
            <div class="ueditor__row">
              <button class="mini" onclick={() => {
                const cells = clone(selectedElement?.grid?.cells ?? []);
                const index = cells.findIndex((cell) => cell.id === selectedCell?.id);
                if (index > 0 && selectedElement) {
                  [cells[index - 1], cells[index]] = [cells[index], cells[index - 1]];
                  updateElement(selectedElement.id, { grid: { ...selectedElement.grid, cells } });
                }
              }}>Sposta prima</button>
              <button class="mini" onclick={() => {
                const cells = clone(selectedElement?.grid?.cells ?? []);
                const index = cells.findIndex((cell) => cell.id === selectedCell?.id);
                if (index >= 0 && index < cells.length - 1 && selectedElement) {
                  [cells[index + 1], cells[index]] = [cells[index], cells[index + 1]];
                  updateElement(selectedElement.id, { grid: { ...selectedElement.grid, cells } });
                }
              }}>Sposta dopo</button>
            </div>
            <div class="ueditor__row">
              <button class="mini" onclick={() => mergeCell("right")}>Unisci a destra</button>
              <button class="mini" onclick={() => mergeCell("down")}>Unisci sotto</button>
              <button class="mini" onclick={splitCell}>Dividi</button>
            </div>
          {/if}
          {#if selectedElement.grid}
            <h4>Griglia · {selectedElement.grid.columns * selectedElement.grid.rows} celle</h4>
            <div class="ueditor__two">
              <label class="ueditor__field"><span>Colonne</span><input type="number" min="1" max="20" value={selectedElement.grid.columns} onchange={(event) => setGridSize(Number(event.currentTarget.value), undefined)} /></label>
              <label class="ueditor__field"><span>Righe</span><input type="number" min="1" max="40" value={selectedElement.grid.rows} onchange={(event) => setGridSize(undefined, Number(event.currentTarget.value))} /></label>
            </div>
            <div class="ueditor__row">
              <button class="mini" onclick={() => resizeGrid(1, 0)}>+ Riga</button>
              <button class="mini" onclick={() => resizeGrid(-1, 0)}>− Riga</button>
              <button class="mini" onclick={() => resizeGrid(0, 1)}>+ Colonna</button>
              <button class="mini" onclick={() => resizeGrid(0, -1)}>− Colonna</button>
              <button class="mini mini--danger" onclick={clearGridCells}>Svuota celle</button>
            </div>
            <label class="ueditor__field"><span>Larghezze colonne (pesi separati da virgola)</span><input bind:value={gridWidthsText} placeholder={Array.from({ length: selectedElement.grid.columns }, () => "1").join(",")} /></label>
            <label class="ueditor__field"><span>Altezze righe (pesi separati da virgola)</span><input bind:value={gridHeightsText} placeholder={Array.from({ length: selectedElement.grid.rows }, () => "1").join(",")} /></label>
            <button class="mini" onclick={applyGridSizes}>Applica dimensioni</button>
          {/if}
        {:else}
          <h3>Documento</h3>
          <p class="hint">Seleziona un elemento sulla pagina per modificarne testo, posizione, pittogramma e azione.</p>
          <label class="ueditor__field"><span>Titolo documento</span><input value={doc.title} onchange={(event) => { onTitleChange?.(event.currentTarget.value); void apply([{ op: "update_document", patch: { title: event.currentTarget.value } }], "Titolo aggiornato."); }} /></label>
          <label class="ueditor__field"><span>Formato foglio</span>
            <select
              value={doc.page.width_mm && doc.page.height_mm
                ? "custom"
                : `${doc.page.size}|${doc.page.orientation}`}
              onchange={(event) => applyStandardFormat(event.currentTarget.value)}
            >
              <option value="A4|portrait">A4 verticale (210×297)</option>
              <option value="A4|landscape">A4 orizzontale (297×210)</option>
              <option value="A3|portrait">A3 verticale (297×420)</option>
              <option value="A3|landscape">A3 orizzontale (420×297)</option>
              <option value="A5|portrait">A5 verticale (148×210)</option>
              <option value="LETTER|portrait">Letter (216×279)</option>
              <option value="16:9|landscape">16:9 (339×191)</option>
              <option value="4:3|landscape">4:3 (254×191)</option>
              <option value="custom">Personalizzato…</option>
            </select>
          </label>
          <label class="ueditor__field"><span>Larghezza (mm)</span>
            <input
              type="number"
              min="50"
              max="2000"
              value={Math.round(page?.width_mm ?? 210)}
              onchange={(event) =>
                applyCustomSize(
                  Number(event.currentTarget.value) || 210,
                  page?.height_mm ?? 297
                )}
            />
          </label>
          <label class="ueditor__field"><span>Margini (mm)</span>
            <input
              type="number"
              min="0"
              max="80"
              step="0.5"
              value={doc.page.margin_mm ?? 15}
              onchange={(event) => void apply([{ op: "update_document", patch: { page: { ...doc.page, margin_mm: Number(event.currentTarget.value) || 0 } } }], "Margini aggiornati.")}
            />
          </label>
          <label class="ueditor__field"><span>Altezza (mm)</span>
            <input
              type="number"
              min="50"
              max="2000"
              value={Math.round(page?.height_mm ?? 297)}
              onchange={(event) =>
                applyCustomSize(
                  page?.width_mm ?? 210,
                  Number(event.currentTarget.value) || 297
                )}
            />
          </label>
          <p class="hint">{docPage?.elements.length ?? 0} elementi · {doc.pages.length} pagine</p>
        {/if}
      </aside>
      {/if}
    {/if}
  </div>
</div>

{#if editingElement && caa}
  <ElementTextEditor
    title={`Testo CAA · ${editingElement.text.slice(0, 40) || editingElement.kind}`}
    text={editingElement.text}
    stored={storedCaaText(editingElement)}
    {caa}
    onCancel={() => (editingTextId = null)}
    onApply={applyElementText}
    onToast={onToast}
  />
{/if}

{#if editingCell && caa}
  <ElementTextEditor
    title={`Frase CAA · cella ${editingCell.id}`}
    text={editingCell.text}
    stored={storedCaaText(editingCell)}
    {caa}
    onCancel={() => (editingCellId = null)}
    onApply={applyCellText}
    onToast={onToast}
  />
{/if}

<style>
  .ueditor { min-height: 100vh; display: flex; flex-direction: column; background: var(--bg, #eef1f6); color: var(--text, #16161a); }
  .ueditor__bar { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding: 8px 12px; background: var(--surface, #fff); border-bottom: 1px solid var(--border, #d3d6e0); position: sticky; top: 0; z-index: 30; }
  .ueditor__title { font-size: 1.05rem; font-weight: 650; min-width: 220px; border: 1px solid transparent; background: transparent; }
  .ueditor__badge { font-size: 0.7rem; padding: 3px 8px; border-radius: 999px; background: #e8eefc; color: #12234a; }
  .grow { flex: 1 1 auto; }
  .ueditor__body { display: grid; grid-template-columns: 170px minmax(0, 1fr) 300px; min-height: 0; flex: 1 1 auto; }
  .ueditor--play .ueditor__body { grid-template-columns: 170px minmax(0, 1fr); }
  .ueditor__body { position: relative; }
  .ueditor__stage { position: relative; }
  .ueditor__pages-reopen { position: absolute; left: 8px; top: 8px; z-index: 25; }
  .ueditor__pages { border-right: 1px solid var(--border, #d3d6e0); background: var(--surface, #fff); padding: 8px; overflow: auto; max-height: calc(100vh - 54px); }
  .ueditor__pages-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
  .thumb { display: block; width: 100%; border: 2px solid transparent; border-radius: 6px; padding: 3px; margin-bottom: 6px; background: transparent; cursor: pointer; color: inherit; }
  .thumb--active { border-color: #2463eb; background: #eef4ff; }
  .thumb__paper { position: relative; display: block; width: 100%; background: #fff; border: 1px solid #cfd6e2; overflow: hidden; }
  .thumb__el { position: absolute; overflow: hidden; display: block; }
  .thumb__el :global(svg) { width: 100%; height: 100%; display: block; }
  .thumb__cell { position: absolute; box-sizing: border-box; border: 0.5px solid #c9d2df; overflow: hidden; display: flex; align-items: center; justify-content: center; }
  .thumb__cell :global(svg) { width: 78%; height: 78%; }
  .thumb__cell-text { font-size: 2.4px; line-height: 1; white-space: nowrap; overflow: hidden; }
  .thumb__text { display: block; font-size: 2.6px; line-height: 1.15; color: #172033; overflow: hidden; padding: 0.5px 1px; }
  .thumb__shape { position: absolute; inset: 0; display: block; box-sizing: border-box; border-style: solid; border-radius: 1px; }
  .thumb__speech { position: absolute; inset: 0; display: flex; align-items: center; justify-content: center; box-sizing: border-box; border: 0.6px solid; font-size: 2.2px; line-height: 1.05; text-align: center; overflow: hidden; padding: 1px; }
  .thumb__title { display: block; font-size: 0.68rem; text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ueditor__page-tools { display: flex; flex-wrap: wrap; gap: 3px; margin: 8px 0; }
  .ueditor__page-title { width: 100%; font-size: 0.78rem; margin-bottom: 6px; }
  .mini { border: 1px solid #cfd6e2; border-radius: 5px; background: #f8fafc; cursor: pointer; font-size: 0.7rem; padding: 2px 5px; color: #1b2330; }
  .mini--danger:hover { background: #fdecec; border-color: #d64545; }
  .ueditor__stage { overflow: auto; padding: 10px 14px 30px; min-width: 0; }
  .ueditor__flowhint { margin: 0 0 8px; font-size: 0.76rem; color: #475569; }
  .ueditor__tools { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; margin-bottom: 8px; }
  .ueditor__gridbox { display: inline-flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 5px 8px; border: 1px solid #d3d6e0; border-radius: 8px; background: #fff; font-size: 0.75rem; }
  .ueditor__gridbox input { width: 54px; }
  .ueditor__zoombar { position: fixed; right: 18px; bottom: 18px; z-index: 45; display: flex; align-items: center; gap: 6px; padding: 5px 8px; background: var(--surface, #fff); border: 1px solid var(--border, #d3d6e0); border-radius: 999px; box-shadow: 0 2px 10px rgba(15,23,42,0.12); font-size: 0.75rem; }
  .ueditor__zoombar input { width: 90px; }
  .ueditor__zoomvalue { min-width: 32px; text-align: right; }
  .ueditor__margin-badge { position: absolute; top: 8px; right: 8px; z-index: 35; background: #172033; color: #fff; font-size: 0.7rem; padding: 2px 7px; border-radius: 999px; }
  .ueditor__pagewrap { display: flex; justify-content: center; }
  .ueditor__textpage { min-width: 0; min-height: 0; overflow: auto; padding: 8px 12px; display: flex; flex-direction: column; }
  .ueditor__textpage .paper :global(.editor) { position: relative; z-index: 1; }
  .ueditor__textpage :global(.caa-editor) { flex: 1 1 auto; min-height: 0; }
  .ueditor__page { position: relative; box-shadow: 0 4px 22px rgba(15,23,42,0.16); overflow: hidden; flex: 0 0 auto; }
  /* Livello degli elementi liberi: sopra il testo della pagina CAA, ma
     trasparente ai clic tranne che sui singoli elementi. */
  .ueditor__freelayer { position: absolute; inset: 0; z-index: 4; pointer-events: none; }
  .ueditor__freelayer .uel { pointer-events: auto; }
  .uel { position: absolute; box-sizing: border-box; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1mm; cursor: move; user-select: none; overflow: hidden; }
  .uel--behind { z-index: 0; }
  .uel--wrap { z-index: 2; }
  .uel--front { z-index: 30; }
  .uel--symbol { cursor: move; }
  .uel--button { cursor: pointer; }
  .uel__token { display: block; width: 78%; aspect-ratio: 1; margin: auto; border: 2px dashed #0f766e; border-radius: 50%; background: #fff; }
  .ueditor--play .uel--toggle.uel--on .uel__token { border-style: solid; background: #fbbf24; }
  .uel--selected { outline: 2px solid #2463eb; outline-offset: 1px; }
  .uel--locked { cursor: default; }
  .ueditor__preview {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 6px 12px;
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
    font-size: 0.84rem;
  }

  .uel__symbol { width: 100%; height: 62%; display: flex; align-items: center; justify-content: center; min-height: 0; }
  .uel__symbol :global(svg), .ueditor__result-media :global(svg), .uel__cell-symbol :global(svg),
  .uel__symbol :global(img), .ueditor__result-media :global(img), .uel__cell-symbol :global(img) { width: 100%; height: 100%; object-fit: contain; }
  .uel--speech {
    overflow: visible;
  }
  .uel--speech::after {
    content: "";
    position: absolute;
    left: 18%;
    bottom: -7px;
    width: 12px;
    height: 12px;
    background: var(--uel-fill, #ffffff);
    border-right: 1.5px solid var(--uel-stroke, #172033);
    border-bottom: 1.5px solid var(--uel-stroke, #172033);
    transform: rotate(45deg) skew(8deg, 8deg);
  }
  .uel__label { font-size: inherit; line-height: 1.15; text-align: center; pointer-events: auto; cursor: text; outline: none; min-width: 2mm; }
  .uel__label[contenteditable="true"]:focus { outline: 1px dashed #2463eb; background: #fff8d8; }
  .uel__caa { font-size: inherit; line-height: 1.2; text-align: center; max-width: 100%; pointer-events: auto; }
  .uel__caa-cell { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 0.4mm; font-size: inherit; line-height: 1.15; max-width: 100%; overflow: hidden; }
  .uel__caa-unit { cursor: pointer; }
  .uel__caa-unit:focus-visible { outline: 2px solid #2463eb; outline-offset: 1px; }
  .uel:has(.uel__caa) { overflow: visible; }
  .uel__grid { position: absolute; inset: 0; }
  .uel__cell { position: absolute; box-sizing: border-box; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 0.5mm; background: #fff; color: #111; font: inherit; font-size: 3.2mm; padding: 0.8mm; cursor: pointer; overflow: hidden; }
  .uel__cell-symbol { width: 60%; height: 55%; }
  .uel__lock { position: absolute; top: 2px; right: 3px; font-size: 0.65rem; }
  .uel__handle { position: absolute; right: -4px; bottom: -4px; width: 12px; height: 12px; border-radius: 50%; background: #2463eb; border: 2px solid #fff; cursor: nwse-resize; }
  .ueditor__properties { border-left: 1px solid var(--border, #d3d6e0); background: var(--surface, #fff); padding: 10px; overflow: auto; max-height: calc(100vh - 54px); }
  .ueditor__properties h3 { margin: 0 0 4px; font-size: 0.95rem; }
  .ueditor__properties h4 { margin: 12px 0 4px; font-size: 0.82rem; }
  .ueditor__field { display: flex; flex-direction: column; gap: 1px; margin-bottom: 5px; font-size: 0.72rem; }
  .ueditor__field input, .ueditor__field select, .ueditor__field textarea { font-size: 0.8rem; padding: 3px 4px; }
  .ueditor__field textarea { min-height: 52px; }
  .ueditor__two { display: grid; grid-template-columns: 1fr 1fr; gap: 5px; }
  .ueditor__row { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; font-size: 0.76rem; }
  .ueditor__row input[type="text"], .ueditor__row input:not([type]) { flex: 1 1 120px; }
  .ueditor__results { display: flex; flex-wrap: wrap; gap: 4px; max-height: 120px; overflow: auto; margin: 4px 0 8px; }
  .ueditor__result { display: flex; align-items: center; gap: 4px; border: 1px solid #cfd6e2; border-radius: 6px; background: #fff; padding: 3px 5px; cursor: pointer; font-size: 0.7rem; }
  .ueditor__result-media { width: 26px; height: 26px; }
  .ueditor__result-thumb { width: 30px; height: 30px; object-fit: contain; }
  .ueditor__result small { color: #667085; font-size: 0.62rem; }
  .ueditor__message { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-top: 10px; padding: 8px; background: #fff; border: 1px solid #d3d6e0; border-radius: 10px; }
  .ueditor__chips { flex: 1 1 240px; display: flex; flex-wrap: wrap; gap: 5px; min-height: 32px; }
  .ueditor__chip { border: 0; border-radius: 999px; background: #e8eefc; color: #12234a; padding: 4px 10px; font-weight: 600; cursor: pointer; }
  .ueditor__message-actions { display: flex; gap: 6px; }
  .ueditor__feedback { width: 100%; margin: 4px 0 0; padding: 6px 10px; border-radius: 8px; background: #fee2e2; color: #991b1b; font-weight: 600; }
  .ueditor__feedback--ok { background: #dcfce7; color: #166534; }
  .ueditor--play .uel--toggle { cursor: pointer; opacity: 0.55; transition: opacity 120ms ease, box-shadow 120ms ease; }
  .ueditor--play .uel--toggle.uel--on { opacity: 1; box-shadow: 0 0 0 2px #0f766e inset; }
  @media (max-width: 1100px) { .ueditor__body { grid-template-columns: 140px minmax(0, 1fr); } .ueditor__properties { display: none; } }
  .ueditor__body--nopages { grid-template-columns: minmax(0, 1fr) 300px; }
  .ueditor__body--noprops { grid-template-columns: 170px minmax(0, 1fr); }
  .ueditor__body--nopages.ueditor__body--noprops { grid-template-columns: minmax(0, 1fr); }
  .ueditor--play .ueditor__body--nopages { grid-template-columns: minmax(0, 1fr); }
  .ueditor__ruler { position: absolute; z-index: 50; pointer-events: none; opacity: 0.7; }
  .ueditor__ruler--top {
    left: 0; right: 0; top: 0; height: 5mm;
    background-image:
      repeating-linear-gradient(to right, #64748b 0 0.18mm, transparent 0.18mm 10mm),
      repeating-linear-gradient(to right, #94a3b8 0 0.12mm, transparent 0.12mm 5mm);
    background-size: 100% 100%, 100% 55%;
    background-position: top left, bottom left;
    background-repeat: no-repeat;
  }
  .ueditor__ruler--left {
    left: 0; top: 0; bottom: 0; width: 5mm;
    background-image:
      repeating-linear-gradient(to bottom, #64748b 0 0.18mm, transparent 0.18mm 10mm),
      repeating-linear-gradient(to bottom, #94a3b8 0 0.12mm, transparent 0.12mm 5mm);
    background-size: 100% 100%, 55% 100%;
    background-position: top left, right top;
    background-repeat: no-repeat;
  }
  .ueditor__margin-line { position: absolute; z-index: 51; background: #2463eb; opacity: 0.5; pointer-events: none; }
  .ueditor__margin-line--left, .ueditor__margin-line--right { top: 0; bottom: 0; width: 0.3mm; }
  .ueditor__margin-line--top, .ueditor__margin-line--bottom { left: 0; right: 0; height: 0.3mm; }
  .ueditor__margin-handle {
    position: absolute; z-index: 52; width: 9px; height: 9px; padding: 0;
    border-radius: 50%; border: 1px solid #fff; background: #2463eb; cursor: ew-resize;
  }
  .ueditor__margin-handle--left, .ueditor__margin-handle--right { top: 50%; transform: translate(-50%, -50%); }
  .ueditor__margin-handle--right { transform: translate(50%, -50%); }
  .ueditor__margin-handle--top, .ueditor__margin-handle--bottom { left: 50%; transform: translate(-50%, -50%); cursor: ns-resize; }
  .ueditor__margin-handle--bottom { transform: translate(-50%, 50%); }
  .ueditor__snap { position: absolute; z-index: 49; background: #e0663a; pointer-events: none; opacity: 0.9; }
  .ueditor__snap--v { top: 0; bottom: 0; width: 0.3mm; }
  .ueditor__snap--h { left: 0; right: 0; height: 0.3mm; }
  @media (max-width: 720px) { .ueditor__body { grid-template-columns: minmax(0, 1fr); } .ueditor__pages { display: none; } }
</style>
