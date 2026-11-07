<script lang="ts">
  import { onMount } from "svelte";
  import { api, isTauri } from "../api";
  import type {
    Alternative,
    Profile,
    RenderedDocument,
    RenderedElement,
    RenderedPage,
    UniversalCommand,
    UniversalDocument,
    UniversalElement,
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
    onExport?: (document: UniversalDocument, format: "html" | "svg" | "pdf" | "png") => Promise<void>;
    canEdit?: boolean;
    onOpen?: () => void;
    onImport?: () => void;
    onEdit?: () => void;
    onToggleFullscreen?: () => void;
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
  let searchBusy = $state(false);
  let history: UniversalDocument[] = [];
  let redoStack: UniversalDocument[] = [];
  let message = $state<{ id: string; text: string; svg: string | null }[]>([]);
  let pageHistory = $state<number[]>([]);
  let zoom = $state(0.78);
  let dirty = $state(false);
  let gridPanel = $state(false);
  let gridColumns = $state(4);
  let gridRows = $state(5);

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

  function localRender(document: UniversalDocument): RenderedDocument {
    const [pageW, pageH] = pageDimensions(document);
    const margin = Math.max(0, document.page.margin_mm);
    const warnings: string[] = [];
    const pages = document.pages.map((item, index) => {
      let flowY = margin + (item.flow.padding_mm ?? 0);
      const flowW = pageW - 2 * margin - 2 * (item.flow.padding_mm ?? 0);
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
          if (item.layout === "flow") {
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
                const cw = (w - gap * (columns - 1)) / columns;
                const ch = (h - gap * (rows - 1)) / rows;
                return element.grid.cells.map((cell, index) => ({
                  id: cell.id,
                  text: cell.text,
                  spoken_text: cell.spoken_text || cell.text,
                  symbol_id: cell.symbol_id ?? null,
                  concept_id: cell.concept_id ?? null,
                  svg: cell.svg ?? null,
                  action: cell.action ?? null,
                  hidden: Boolean(cell.hidden),
                  x_mm: (index % columns) * (cw + gap),
                  y_mm: Math.floor(index / columns) * (ch + gap),
                  w_mm: cw,
                  h_mm: ch,
                  style: cell.style ?? {},
                }));
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
            action: element.action ?? null,
            link_page: element.link_page ?? null,
            group_id: element.group_id ?? null,
            cells,
            accessibility: element.accessibility ?? {},
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
      dirty = true;
      if (!doc.pages[pageIndex]) pageIndex = Math.max(0, doc.pages.length - 1);
      if (label) onToast(label, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      busy = false;
    }
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
    void apply(
      [{ op: "update_element", page: docPage.id, id, patch }],
      "Elemento aggiornato."
    );
  }

  function setStyle(patch: Partial<UniversalStyle>) {
    if (!docPage || !selectedId) return;
    void apply(
      [{ op: "set_style", page: docPage.id, id: selectedId, style: patch }],
      "Stile aggiornato."
    );
  }

  function setAction(patch: Record<string, unknown> | null) {
    if (!docPage || !selectedId) return;
    const action = patch
      ? { type: String(patch.type ?? "append"), target_page: (patch.target_page as string) || null, target_document: null, url: null }
      : null;
    void apply(
      [{ op: "set_action", page: docPage.id, id: selectedId, action }],
      "Azione aggiornata."
    );
  }

  function setText(value: string, spoken = false) {
    if (!docPage || !selectedId) return;
    if (spoken) {
      updateElement(selectedId, { spoken_text: value });
    } else {
      void apply(
        [{ op: "set_text", page: docPage.id, id: selectedId, text: value }],
        "Testo aggiornato."
      );
    }
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
    const id = `el-${Date.now().toString(36)}`;
    const element: UniversalElement = {
      id,
      kind,
      text: kind === "text" || kind === "text_box" || kind === "button" ? "Nuovo testo" : "",
      spoken_text: "",
      x_mm: Math.max(5, (doc.page.margin_mm || 12) + 5),
      y_mm: Math.max(5, (doc.page.margin_mm || 12) + 5),
      w_mm: kind === "button" ? 45 : 80,
      h_mm: kind === "button" ? 22 : kind === "shape" ? 30 : 20,
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
          ? { font_size: 4.5, border_width: 1, border_color: "#456", align: "center" }
          : { font_size: 4.5 },
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
    try {
      const result = await api.searchSymbols(query, 18, 0, null, false);
      searchResults = result.items;
    } finally {
      searchBusy = false;
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
        if (!target.text.trim()) target.text = alternative.label;
        if (!target.spoken_text.trim()) target.spoken_text = alternative.label;
        updateElement(selectedElement.id, {
          grid: { ...selectedElement.grid, cells },
        });
      }
    } else {
      void apply(
        [{
          op: "set_symbol",
          page: docPage.id,
          id: selectedId,
          symbol_id: alternative.symbol_id,
          concept_id: alternative.concept_id,
          svg: alternative.svg,
          label: alternative.label,
        }],
        "Pittogramma sostituito."
      );
    }
    searchResults = [];
    searchQuery = "";
  }

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
    if (!confirm(`Eliminare la pagina «${docPage.title}»?`)) return;
    void apply([{ op: "remove_page", id: docPage.id }], "Pagina eliminata.");
    pageIndex = Math.max(0, pageIndex - 1);
  }

  function movePage(delta: number) {
    if (!docPage) return;
    const target = pageIndex + delta;
    if (target < 0 || target >= doc.pages.length) return;
    void apply([{ op: "move_page", id: docPage.id, index: target }], "Pagina spostata.");
    pageIndex = target;
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
  }

  function playAction(element: RenderedElement | { action?: any; spoken_text?: string; text?: string }) {
    const action = element.action?.type ?? "append";
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

  function startDrag(event: PointerEvent, element: RenderedElement, resize = false) {
    if (mode !== "edit" || element.locked || readOnly) return;
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
    const dx = pxToMm(event.clientX - drag.startX);
    const dy = pxToMm(event.clientY - drag.startY);
    if (drag.mode === "move") {
      dragPreview = {
        id: drag.id,
        x: Math.max(0, drag.originalX + dx),
        y: Math.max(0, drag.originalY + dy),
        w: drag.originalW,
        h: drag.originalH,
      };
    } else {
      dragPreview = {
        id: drag.id,
        x: drag.originalX,
        y: drag.originalY,
        w: Math.max(5, drag.originalW + dx),
        h: Math.max(5, drag.originalH + dy),
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
      `border:${border}`,
      `border-radius:${element.style.border_radius ?? 2}mm`,
      `color:${element.style.color || "#111"}`,
      `font-size:${element.style.font_size ?? 4.2}mm`,
      `text-align:${element.style.align || "center"}`,
      `padding:${element.style.padding_mm ?? 1}mm`,
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
    try {
      const html = isTauri()
        ? await api.documentHtml(doc)
        : "<html><body>La stampa richiede l'app desktop.</body></html>";
      const frame = document.createElement("iframe");
      frame.setAttribute("aria-hidden", "true");
      frame.style.position = "fixed";
      frame.style.width = "1px";
      frame.style.height = "1px";
      frame.style.opacity = "0";
      frame.style.pointerEvents = "none";
      frame.srcdoc = html;
      frame.onload = () => {
        try {
          frame.contentWindow?.focus();
          frame.contentWindow?.print();
        } finally {
          window.setTimeout(() => frame.remove(), 2000);
        }
      };
      document.body.append(frame);
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function save() {
    saving = true;
    void onSave(doc).finally(() => (saving = false));
  }
</script>

<svelte:window on:keydown={handleKey} />

<div class="ueditor" class:ueditor--play={mode === "play"} data-profile-size={profile?.accessibility?.large_symbols ? "large" : "normal"}>
  <header class="ueditor__bar">
    <button class="btn" onclick={onClose}>← Home</button>
    <input class="ueditor__title" value={doc.title} aria-label="Titolo del documento" onchange={(event) => void apply([{ op: "update_document", patch: { title: event.currentTarget.value } }], "Titolo aggiornato.")} readonly={mode === "play"} />
    <span class="ueditor__badge">{doc.kind}</span>
    <span class="ueditor__badge">{doc.pages.length} pagine</span>
    <span class="grow"></span>
    {#if mode === "edit"}
      <button class="btn" disabled={busy || history.length === 0} onclick={undo}>Annulla</button>
      <button class="btn" disabled={busy || redoStack.length === 0} onclick={redo}>Ripeti</button>
      <button class="btn" onclick={printDocument}>Stampa</button>
      <button class="btn" onclick={() => (mode = "play")}>Anteprima</button>
      <button class="btn" onclick={save} disabled={saving}>{saving ? "Salvo…" : "Salva .caadoc"}</button>
      {#if onSavePlayer}
        <button class="btn" onclick={() => void onSavePlayer(doc)}>Salva .caaplay</button>
      {/if}
      {#if onExport}
        <button class="btn" onclick={() => void onExport(doc, "html")}>HTML</button>
        <button class="btn" onclick={() => void onExport(doc, "pdf")}>PDF</button>
        <button class="btn" onclick={() => void onExport(doc, "png")}>PNG</button>
      {/if}
    {:else}
      <button class="btn" onclick={onOpen}>Apri</button>
      <button class="btn" onclick={onImport}>Importa</button>
      {#if canEdit}
        <button class="btn" onclick={onEdit}>Modifica</button>
      {/if}
      <button class="btn" onclick={printDocument}>Stampa</button>
      {#if onToggleFullscreen}
        <button class="btn" onclick={onToggleFullscreen}>Schermo intero</button>
      {/if}
      <button class="btn" onclick={() => (pageHistory = [...pageHistory, pageIndex], pageIndex = Math.max(0, doc.pages.findIndex((p) => p.is_home)))}>Home pagina</button>
    {/if}
  </header>

  <div class="ueditor__body">
    <aside class="ueditor__pages" aria-label="Pagine">
      <div class="ueditor__pages-head">
        <strong>Pagine</strong>
        {#if mode === "edit"}
          <button class="mini" onclick={addPage}>+</button>
        {/if}
      </div>
      {#each rendered.pages as item (item.id)}
        <button
          class="thumb"
          class:thumb--active={item.index === pageIndex}
          onclick={() => (pageIndex = item.index)}
          title={item.title}
        >
          <span class="thumb__paper" style={`aspect-ratio:${item.width_mm}/${item.height_mm}`}>
            {#each item.elements.slice(0, 20) as element (element.id)}
              <span
                class="thumb__block"
                style={`left:${(element.x_mm / item.width_mm) * 100}%;top:${(element.y_mm / item.height_mm) * 100}%;width:${(element.w_mm / item.width_mm) * 100}%;height:${(element.h_mm / item.height_mm) * 100}%`}
              ></span>
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
          <select value={docPage.layout} onchange={(event) => setPageLayout(event.currentTarget.value)}>
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

    <main class="ueditor__stage" class:ueditor__stage--play={mode === "play"}>
      {#if mode === "edit"}
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
          <label class="ueditor__zoom">Zoom <input type="range" min="0.45" max="1.3" step="0.05" bind:value={zoom} /></label>
        </div>
      {/if}
      {#if page}
        <div class="ueditor__pagewrap" style={`zoom:${zoom}`}>
          <div
            class="ueditor__page"
            style={`width:${page.width_mm}mm;height:${page.height_mm}mm;background:${page.background || "#fff"}`}
            role={mode === "play" ? "group" : "application"}
            aria-label={`${page.title} (${page.width_mm}×${page.height_mm} mm)`}
          >
            {#each page.elements as element (element.id)}
              {#if element.visible}
                <div
                  class="uel"
                  class:uel--selected={(selectedId === element.id || extraSelected.includes(element.id)) && mode === "edit"}
                  class:uel--locked={element.locked}
                  class:uel--symbol={element.kind === "symbol"}
                  class:uel--button={element.kind === "button"}
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
                  {#if element.kind === "button" || element.kind === "shape"}
                    <span class="uel__label">{element.text}</span>
                  {:else if element.svg}
                    <span class="uel__symbol">{@html element.svg}</span>
                    {#if element.text}<span class="uel__label">{element.text}</span>{/if}
                  {:else if element.cells?.length}
                    <span class="uel__grid">
                      {#each element.cells as cell (cell.id)}
                        {#if !cell.hidden}
                          <button
                            class="uel__cell"
                            style={`left:${cell.x_mm}mm;top:${cell.y_mm}mm;width:${cell.w_mm}mm;height:${cell.h_mm}mm;background:${cell.style.background || "transparent"};border:${cell.style.border_width ? `1px solid ${cell.style.border_color || "#9aa7bb"}` : "1px solid #d0d7e2"}`}
                            onpointerdown={(event) => event.stopPropagation()}
                            onclick={(event) => {
                              if (mode === "play") playCell(cell, event);
                              else {
                                event.stopPropagation();
                                selectedId = element.id;
                                selectedCellId = cell.id;
                              }
                            }}
                          >
                            {#if cell.svg}<span class="uel__cell-symbol">{@html cell.svg}</span>{/if}
                            <span>{cell.text}</span>
                          </button>
                        {/if}
                      {/each}
                    </span>
                  {:else}
                    <span class="uel__label">{element.text || element.kind}</span>
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
          <div class="ueditor__message-actions">
            <button class="btn" onclick={() => (message = [])} disabled={message.length === 0}>Svuota</button>
            <button class="btn btn--solid" onclick={speakMessage} disabled={message.length === 0}>Parla frase</button>
          </div>
        </div>
      {/if}
    </main>

    {#if mode === "edit"}
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
          </div>
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
          <h4>Pittogramma</h4>
          <div class="ueditor__row">
            <input bind:value={searchQuery} placeholder="Cerca un simbolo…" onkeydown={(event) => { if (event.key === "Enter") { event.preventDefault(); void searchSymbol(); } }} />
            <button class="btn" onclick={() => void searchSymbol()} disabled={searchBusy}>Cerca</button>
          </div>
          {#if searchResults.length > 0}
            <div class="ueditor__results">
              {#each searchResults as alternative (alternative.symbol_id)}
                <button class="ueditor__result" onclick={() => applySymbol(alternative)} title={alternative.license}>
                  {#if alternative.svg}<span class="ueditor__result-media">{@html alternative.svg}</span>{/if}
                  <span>{alternative.label}</span>
                </button>
              {/each}
            </div>
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
          {/if}
        {:else}
          <h3>Documento</h3>
          <p class="hint">Seleziona un elemento sulla pagina per modificarne testo, posizione, pittogramma e azione.</p>
          <label class="ueditor__field"><span>Titolo documento</span><input value={doc.title} onchange={(event) => void apply([{ op: "update_document", patch: { title: event.currentTarget.value } }], "Titolo aggiornato.")} /></label>
          <label class="ueditor__field"><span>Formato</span>
            <select value={`${doc.page.size}-${doc.page.orientation}`} onchange={(event) => {
              const [size, orientation] = event.currentTarget.value.split("-");
              void apply(
                [{ op: "update_document", patch: { page: { ...doc.page, size, orientation } } }],
                "Formato aggiornato."
              );
            }}>
              <option value="A4-portrait">A4 verticale</option>
              <option value="A4-landscape">A4 orizzontale</option>
              <option value="A3-portrait">A3 verticale</option>
              <option value="A3-landscape">A3 orizzontale</option>
              <option value="16:9-landscape">16:9</option>
              <option value="4:3-landscape">4:3</option>
              <option value="LETTER-portrait">Letter</option>
            </select>
          </label>
          <p class="hint">{docPage?.elements.length ?? 0} elementi · {doc.pages.length} pagine</p>
        {/if}
      </aside>
    {/if}
  </div>
</div>

<style>
  .ueditor { min-height: 100vh; display: flex; flex-direction: column; background: var(--bg, #eef1f6); color: var(--text, #16161a); }
  .ueditor__bar { display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding: 8px 12px; background: var(--surface, #fff); border-bottom: 1px solid var(--border, #d3d6e0); position: sticky; top: 0; z-index: 30; }
  .ueditor__title { font-size: 1.05rem; font-weight: 650; min-width: 220px; border: 1px solid transparent; background: transparent; }
  .ueditor__badge { font-size: 0.7rem; padding: 3px 8px; border-radius: 999px; background: #e8eefc; color: #12234a; }
  .grow { flex: 1 1 auto; }
  .ueditor__body { display: grid; grid-template-columns: 170px minmax(0, 1fr) 300px; min-height: 0; flex: 1 1 auto; }
  .ueditor--play .ueditor__body { grid-template-columns: 170px minmax(0, 1fr); }
  .ueditor__pages { border-right: 1px solid var(--border, #d3d6e0); background: var(--surface, #fff); padding: 8px; overflow: auto; max-height: calc(100vh - 54px); }
  .ueditor__pages-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
  .thumb { display: block; width: 100%; border: 2px solid transparent; border-radius: 6px; padding: 3px; margin-bottom: 6px; background: transparent; cursor: pointer; color: inherit; }
  .thumb--active { border-color: #2463eb; background: #eef4ff; }
  .thumb__paper { position: relative; display: block; width: 100%; background: #fff; border: 1px solid #cfd6e2; overflow: hidden; }
  .thumb__block { position: absolute; background: #b9c6da; border-radius: 1px; }
  .thumb__title { display: block; font-size: 0.68rem; text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .ueditor__page-tools { display: flex; flex-wrap: wrap; gap: 3px; margin: 8px 0; }
  .ueditor__page-title { width: 100%; font-size: 0.78rem; margin-bottom: 6px; }
  .mini { border: 1px solid #cfd6e2; border-radius: 5px; background: #f8fafc; cursor: pointer; font-size: 0.7rem; padding: 2px 5px; color: #1b2330; }
  .mini--danger:hover { background: #fdecec; border-color: #d64545; }
  .ueditor__stage { overflow: auto; padding: 10px 14px 30px; min-width: 0; }
  .ueditor__tools { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; margin-bottom: 8px; }
  .ueditor__gridbox { display: inline-flex; flex-wrap: wrap; align-items: center; gap: 6px; padding: 5px 8px; border: 1px solid #d3d6e0; border-radius: 8px; background: #fff; font-size: 0.75rem; }
  .ueditor__gridbox input { width: 54px; }
  .ueditor__zoom { display: inline-flex; align-items: center; gap: 4px; font-size: 0.75rem; }
  .ueditor__zoom input { width: 90px; }
  .ueditor__pagewrap { display: flex; justify-content: center; }
  .ueditor__page { position: relative; box-shadow: 0 4px 22px rgba(15,23,42,0.16); overflow: hidden; flex: 0 0 auto; }
  .uel { position: absolute; box-sizing: border-box; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1mm; cursor: move; user-select: none; overflow: hidden; }
  .uel--symbol { cursor: move; }
  .uel--button { cursor: pointer; }
  .uel--selected { outline: 2px solid #2463eb; outline-offset: 1px; }
  .uel--locked { cursor: default; }
  .uel__symbol { width: 100%; height: 62%; display: flex; align-items: center; justify-content: center; min-height: 0; }
  .uel__symbol :global(svg), .ueditor__result-media :global(svg), .uel__cell-symbol :global(svg) { width: 100%; height: 100%; }
  .uel__label { font-size: inherit; line-height: 1.15; text-align: center; pointer-events: none; }
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
  .ueditor__message { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-top: 10px; padding: 8px; background: #fff; border: 1px solid #d3d6e0; border-radius: 10px; }
  .ueditor__chips { flex: 1 1 240px; display: flex; flex-wrap: wrap; gap: 5px; min-height: 32px; }
  .ueditor__chip { border: 0; border-radius: 999px; background: #e8eefc; color: #12234a; padding: 4px 10px; font-weight: 600; cursor: pointer; }
  .ueditor__message-actions { display: flex; gap: 6px; }
  @media (max-width: 1100px) { .ueditor__body { grid-template-columns: 140px minmax(0, 1fr); } .ueditor__properties { display: none; } }
  @media (max-width: 720px) { .ueditor__body { grid-template-columns: minmax(0, 1fr); } .ueditor__pages { display: none; } }
</style>
