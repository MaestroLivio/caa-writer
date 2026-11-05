<script lang="ts">
  import { onMount } from "svelte";
  import { EditorState, Plugin, TextSelection } from "prosemirror-state";
  import type { Transaction } from "prosemirror-state";
  import { EditorView } from "prosemirror-view";
  import type { NodeView, ViewMutationRecord } from "prosemirror-view";
  import type { Mark, Node as PMNode } from "prosemirror-model";
  import { history, redoDepth, undo, redo, undoDepth } from "prosemirror-history";
  import { keymap } from "prosemirror-keymap";
  import {
    baseKeymap,
    selectAll,
    setBlockType,
    splitBlock,
    toggleMark,
  } from "prosemirror-commands";
  import {
    addColumnAfter,
    addRowAfter,
    deleteColumn,
    deleteRow,
    isInTable,
    mergeCells,
    splitCell,
  } from "prosemirror-tables";
  import {
    liftListItem,
    sinkListItem,
    splitListItem,
    wrapInList,
  } from "prosemirror-schema-list";
  import { schema } from "./schema";
  import { tokenForSelection } from "./selection";
  import { completedSentenceBefore, lastSentence } from "./sentence";
  import type { FormattingState } from "./formatting";
  import { api, demoAnalyze, isTauri } from "../api";
  import type { CaaMode, CaaToken, Intensity } from "../types";

  interface Props {
    mode: CaaMode;
    intensity: Intensity;
    documentKey: string;
    commercialOnly: boolean;
    /// Didascalia del repertorio sotto il pittogramma. Spenta per default:
    /// la parola scritta è già nel testo sotto la striscia.
    showCaptions: boolean;
    /// Bordo del pittogramma (nessuno, immagine, immagine+testo, separato, divisore).
    borderStyle: string;
    packId: string | null;
    selectedUnit: { pid: string; index: number } | null;
    /// Simbolizzazione automatica: se disattivata i pittogrammi restano memorizzati
    /// ma non vengono ricalcolati né mostrati.
    autoEnabled: boolean;
    /// Reranking contestuale con il modello locale (già verificato come disponibile).
    neural: boolean;
    /// Simbolizzazione delle parole grammaticali, per categoria.
    symbolizeArticles: boolean;
    symbolizePrepositions: boolean;
    symbolizeConjunctions: boolean;
    onSelectToken: (pid: string, token: CaaToken) => void;
    /// Menu contestuale del pittogramma (blocco, visibilità, modificatori).
    onTokenMenu: (pid: string, token: CaaToken, at: { x: number; y: number }) => void;
    onError: (message: string) => void;
    onAnalyzed: (byParagraph: Map<string, CaaToken[]>) => void;
    /// Stato di formattazione della selezione: serve al Ribbon per mostrare
    /// i pulsanti attivi (grassetto, corsivo, allineamento, tabella...).
    onFormattingChange?: (state: FormattingState) => void;
    /// Vero quando il testo del documento cambia (non le sole associazioni).
    onDocChanged?: () => void;
  }

  let {
    mode,
    intensity,
    documentKey,
    commercialOnly,
    showCaptions,
    borderStyle,
    packId,
    selectedUnit,
    autoEnabled,
    neural,
    symbolizeArticles,
    symbolizePrepositions,
    symbolizeConjunctions,
    onSelectToken,
    onTokenMenu,
    onError,
    onAnalyzed,
    onFormattingChange,
    onDocChanged,
  }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;

  const tokensByPid = new Map<string, CaaToken[]>();
  const analyzedSignature = new Map<string, string>();
  const paragraphViews = new Map<string, TextView>();
  let debounce: ReturnType<typeof setTimeout> | null = null;
  let analyzing = false;
  let queued = false;
  let refineTimer: ReturnType<typeof setTimeout> | null = null;
  let refineSeq = 0;
  const refinedSignature = new Map<string, string>();

  const newPid = (): string =>
    `p${Date.now().toString(36)}${Math.random().toString(36).slice(2, 7)}`;

  interface ParagraphInfo {
    pid: string;
    text: string;
    style: string;
    merges: { start: number; end: number }[];
    splits: { start: number; end: number }[];
  }

  function signatureFor(paragraph: ParagraphInfo): string {
    return `${paragraph.text}\u0001${mode}\u0001${intensity}\u0001${commercialOnly}\u0001${packId ?? ""}\u0001${documentKey}\u0001${symbolizeArticles}\u0001${symbolizePrepositions}\u0001${symbolizeConjunctions}\u0001${JSON.stringify(paragraph.merges)}\u0001${JSON.stringify(paragraph.splits)}`;
  }

  function modifierList(token: CaaToken): string[] {
    const out: string[] = [];
    if (token.modifiers?.plural) out.push("plural");
    if (token.modifiers?.negation) out.push("negation");
    if (token.modifiers?.past) out.push("past");
    if (token.modifiers?.future) out.push("future");
    return out;
  }

  /// Token che producono un elemento visibile nella striscia: simbolo,
  /// composizione o almeno parola chiave (le assenze restano segnalate con «?»).
  function stripTokens(tokens: CaaToken[]): CaaToken[] {
    return tokens.filter((token) => {
      if (token.is_punct) return false;
      const hasMedia = Boolean(token.symbol_id) || token.composition.length > 0;
      // Le parole grammaticali senza pittogramma (articoli, copula, preposizioni)
      // non si simbolizzano in CAA: meglio nessun chip di un «?» incomprensibile.
      if (!hasMedia && token.is_function) return false;
      return hasMedia || token.key;
    });
  }

  function renderChips(strip: HTMLElement, pid: string, tokens: CaaToken[]) {
    strip.replaceChildren();
    for (const token of tokens) {
      const hasMedia = Boolean(token.symbol_id) || token.composition.length > 0;
      const button = document.createElement("button");
      button.type = "button";
      button.className = "caa-chip";
      if (token.approximate) button.classList.add("caa-chip--approx");
      if (!hasMedia) button.classList.add("caa-chip--missing");
      // Pittogramma nascosto: resta un segnaposto in schermo per poterlo
      // mostrare di nuovo, ma non compare nella stampa né nelle esportazioni.
      if (token.hidden) {
        button.classList.add("caa-chip--hidden");
        button.dataset.hidden = "true";
      }
      if (token.locked) {
        button.dataset.locked = "true";
      }
      const mods = modifierList(token);
      if (mods.length > 0) {
        button.dataset.modifiers = mods.join(" ");
      }
      if (selectedUnit && selectedUnit.pid === pid && selectedUnit.index === token.index) {
        button.classList.add("caa-chip--selected");
        button.setAttribute("aria-current", "true");
      }
      if (appliedKey === `${pid}:${token.index}`) {
        button.classList.add("caa-chip--applied");
      }
      button.dataset.index = String(token.index);
      // Segnalazione discreta: il modello ha proposto simboli affini per questa
      // parola. Il segno è disegnato via CSS (::after), senza nodi di testo
      // dentro la striscia: ProseMirror non può assimilarlo al documento.
      const hasNeural = token.alternatives.some((alternative) => alternative.neural);
      if (hasNeural) button.dataset.neural = "true";

      const media = document.createElement("span");
      media.className = "caa-chip__media";
      if (token.svg && token.svg.trimStart().startsWith("<svg")) {
        media.innerHTML = token.svg;
      } else if (token.composition.length > 0) {
        media.classList.add("caa-chip__media--composition");
        for (const part of token.composition) {
          const partSpan = document.createElement("span");
          if (part.svg && part.svg.trimStart().startsWith("<svg")) partSpan.innerHTML = part.svg;
          partSpan.title = part.label;
          media.append(partSpan);
        }
      } else {
        // Nessun pittogramma: al posto del «?» si mostra la parola stessa, così
        // la striscia resta leggibile e non sembra un errore. Il testo è reso
        // via CSS (attr) per non finire nel documento.
        const empty = document.createElement("span");
        empty.className = "caa-chip__empty";
        empty.dataset.word = token.surface;
        media.append(empty);
      }

      // La didascalia è generata via CSS (attr) e non è un nodo di testo: se
      // ProseMirror rilegge il blocco non può assimilarla al documento.
      const caption = document.createElement("span");
      caption.className = "caa-chip__caption";
      // Sotto il pittogramma si mostra la parola scritta dall'utente, non
      // l'etichetta del repertorio: quella resta nei metadati (tooltip e pannello).
      caption.dataset.caption = token.surface;

      button.append(media, caption);
      if (token.locked) {
        const lock = document.createElement("span");
        lock.className = "caa-chip__lock";
        button.append(lock);
      }
      if (mods.length > 0) {
        const badges = document.createElement("span");
        badges.className = "caa-chip__mods";
        for (const kind of mods) {
          const badge = document.createElement("span");
          badge.className = `caa-chip__mod caa-chip__mod--${kind}`;
          badges.append(badge);
        }
        button.append(badges);
      }
      const tooltip = [
        token.symbol_label ?? token.concept_label,
        token.meaning,
        token.strategy !== "exact" && token.strategy !== "none"
          ? `strategia: ${token.strategy}`
          : null,
        hasNeural ? "simboli affini proposti dal modello locale" : null,
        "Clic per cambiare pittogramma",
      ]
        .filter(Boolean)
        .join(" — ");
      button.title = tooltip;
      button.setAttribute(
        "aria-label",
        `${token.surface}: ${token.symbol_label ?? token.concept_label ?? token.caption}${token.approximate ? " (pittogramma approssimativo)" : ""}${
          hasNeural ? "; sono disponibili simboli affini suggeriti dal modello locale" : ""
        }. Apri le alternative.`
      );
      button.addEventListener("mousedown", (event) => event.preventDefault());
      button.addEventListener("click", (event) => {
        event.preventDefault();
        onSelectToken(pid, token);
      });
      button.addEventListener("contextmenu", (event) => {
        event.preventDefault();
        onSelectToken(pid, token);
        onTokenMenu(pid, token, { x: event.clientX, y: event.clientY });
      });
      strip.append(button);
    }
  }

  /// Vista di paragrafo/titolo: la striscia sta FUORI dal `contentDOM`, quindi
  /// ProseMirror la tratta come parte della vista e non come contenuto: non la
  /// rimuove durante la digitazione e non ne legge il testo nel documento.
  class TextView implements NodeView {
    dom: HTMLElement;
    contentDOM: HTMLElement;
    strip: HTMLElement;
    private node: PMNode;
    private pid: string;
    private tag: "paragraph" | "heading";
    private layout: "sopra" | "sotto" | null = null;

    constructor(node: PMNode, tag: "paragraph" | "heading") {
      this.node = node;
      this.tag = tag;
      this.pid = String(node.attrs.pid ?? "");
      this.dom = document.createElement("div");
      this.dom.className = "caa-block";
      this.dom.dataset.pid = this.pid;
      this.contentDOM = document.createElement(tag === "heading" ? `h${node.attrs.level ?? 1}` : "p");
      this.contentDOM.className = "caa-text";
      // ProseMirror rilegge il DOM del blocco mentre si digita: senza data-pid
      // qui il pid andrebbe perso e i pittogrammi non troverebbero più i token.
      this.contentDOM.setAttribute("data-pid", this.pid);
      this.strip = document.createElement("div");
      this.strip.className = "caa-strip";
      this.strip.setAttribute("contenteditable", "false");
      this.strip.setAttribute("role", "group");
      this.strip.setAttribute("aria-label", "Pittogrammi del paragrafo");
      this.applyAttrs();
      this.renderStrip();
      paragraphViews.set(this.pid, this);
    }

    private applyAttrs() {
      const attrs = this.node.attrs;
      const indent = Number(attrs.indent ?? 0);
      this.contentDOM.style.textAlign = (attrs.align as string | null) ?? "";
      this.contentDOM.style.paddingLeft = indent > 0 ? `${indent * 24}px` : "";
      this.contentDOM.style.lineHeight = (attrs.line_height as string | null) ?? "";
      this.contentDOM.style.marginTop = (attrs.space_before as string | null) ?? "";
      this.contentDOM.style.marginBottom = (attrs.space_after as string | null) ?? "";
      if (attrs.border) {
        this.contentDOM.style.border = "1px solid var(--border)";
        this.contentDOM.style.borderRadius = "6px";
        this.contentDOM.style.paddingTop = "6px";
        this.contentDOM.style.paddingBottom = "6px";
        this.contentDOM.style.paddingRight = "10px";
      } else {
        this.contentDOM.style.border = "";
        this.contentDOM.style.borderRadius = "";
        this.contentDOM.style.paddingTop = "";
        this.contentDOM.style.paddingBottom = "";
        this.contentDOM.style.paddingRight = "";
      }
      this.dom.classList.toggle("caa-block--break", Boolean(attrs.page_break));
      this.dom.dataset.pid = this.pid;
      this.contentDOM.setAttribute("data-pid", this.pid);
    }

    /// Il pid può cambiare se ProseMirror ricostruisce il nodo: la vista deve
    /// seguirne il valore, altrimenti resta collegata a un paragrafo inesistente.
    private syncPid() {
      const next = String(this.node.attrs.pid ?? "");
      if (next === this.pid) return;
      if (paragraphViews.get(this.pid) === this) paragraphViews.delete(this.pid);
      this.pid = next;
      if (next) paragraphViews.set(next, this);
    }

    /// Riordina i figli solo quando la disposizione cambia davvero: spostare i
    /// nodi a ogni aggiornamento produrrebbe mutazioni DOM che ProseMirror
    /// rileggerebbe come contenuto del documento.
    private order() {
      const next: "sopra" | "sotto" = mode === "sotto" ? "sotto" : "sopra";
      if (this.layout === next) return;
      this.layout = next;
      if (next === "sotto") this.dom.append(this.contentDOM, this.strip);
      else this.dom.append(this.strip, this.contentDOM);
    }

    renderStrip() {
      const visible = stripTokens(tokensByPid.get(this.pid) ?? []);
      const hidden = !autoEnabled || mode === "testo" || visible.length === 0;
      if (hidden) {
        this.strip.replaceChildren();
        this.strip.hidden = true;
      } else {
        this.strip.hidden = false;
        this.strip.dataset.layout = mode === "sotto" ? "sotto" : "sopra";
        renderChips(this.strip, this.pid, visible);
      }
      this.order();
    }

    update(node: PMNode): boolean {
      if (node.type !== this.node.type) return false;
      if (this.tag === "heading" && node.attrs.level !== this.node.attrs.level) return false;
      this.node = node;
      this.syncPid();
      this.applyAttrs();
      this.renderStrip();
      return true;
    }

    ignoreMutation(mutation: ViewMutationRecord): boolean {
      // Mutazioni sul wrapper (riordino striscia) e dentro la striscia non sono
      // contenuto: senza questo filtro ProseMirror ne legge il testo nel documento.
      if (mutation.target === this.dom) return true;
      return this.strip.contains(mutation.target as Node);
    }

    stopEvent(event: Event): boolean {
      return this.strip.contains(event.target as Node);
    }

    destroy() {
      if (paragraphViews.get(this.pid) === this) paragraphViews.delete(this.pid);
    }
  }

  function textViewFor(tag: "paragraph" | "heading") {
    return (node: PMNode) => new TextView(node, tag);
  }

  function refreshStrips() {
    for (const paragraphView of paragraphViews.values()) paragraphView.renderStrip();
  }

  // La selezione testuale è la fonte principale della scelta: un clic su una
  // parola senza pittogramma deve aprire il pannello, non essere ignorato.
  // Si sincronizza solo dopo un gesto dell'utente (mouseup o frecce), mai a
  // ogni tasto: altrimenti il pannello cambierebbe durante la digitazione.
  let lastTextSelectionKey = "";
  // Ultimo intervallo selezionato (offset nel blocco): serve ai comandi
  // «Unisci» anche dopo che il focus è passato al Ribbon.
  let lastBlockRange: { pid: string; from: number; to: number } | null = null;
  // Solo il pittogramma appena applicato riceve l'animazione di comparsa:
  // la digitazione normale non deve animare ogni chip.
  let appliedKey = "";

  function flashApplied(pid: string, index: number) {
    const key = `${pid}:${index}`;
    appliedKey = key;
    window.setTimeout(() => {
      if (appliedKey === key) {
        appliedKey = "";
        refreshStrips();
      }
    }, 240);
  }

  let lastFormatting = "";

  function reportFormatting() {
    if (!view || !onFormattingChange) return;
    const state = view.state;
    const marks = state.storedMarks ?? state.selection.$from.marks();
    const has = (name: string) => marks.some((mark) => mark.type.name === name);
    const style = marks.find((mark) => mark.type.name === "text_style");
    const parent = state.selection.$from.parent;
    let inList = false;
    for (let depth = state.selection.$from.depth; depth > 0; depth -= 1) {
      const node = state.selection.$from.node(depth);
      if (node.type.name === "bullet_list" || node.type.name === "ordered_list") {
        inList = true;
      }
    }
    const next = {
      bold: has("strong"),
      italic: has("em"),
      underline: has("underline"),
      strike: has("strike"),
      highlight: has("highlight"),
      color: style?.attrs.color ?? null,
      fontSize: style?.attrs.font_size ?? null,
      align: (parent.attrs.align as string | null) ?? null,
      heading: parent.type.name === "heading" ? Number(parent.attrs.level) : null,
      indent: Number(parent.attrs.indent ?? 0),
      lineHeight: (parent.attrs.line_height as string | null) ?? null,
      paragraphBorder: Boolean(parent.attrs.border),
      inTable: isInTable(state),
      inList,
      canUndo: undoDepth(state) > 0,
      canRedo: redoDepth(state) > 0,
    };
    const signature = JSON.stringify(next);
    if (signature === lastFormatting) return;
    lastFormatting = signature;
    onFormattingChange(next);
  }

  function selectionFromView(current: EditorView) {
    reportFormatting();
    const { from, to } = current.state.selection;
    const resolved = current.state.doc.resolve(from);
    let depth = resolved.depth;
    while (
      depth > 0 &&
      resolved.node(depth).type.name !== "paragraph" &&
      resolved.node(depth).type.name !== "heading"
    ) {
      depth -= 1;
    }
    if (depth === 0) return;
    const block = resolved.node(depth);
    const pid = String(block.attrs.pid ?? "");
    if (!pid) return;
    const start = resolved.start(depth);
    const end = Math.min(to, resolved.end(depth));
    lastBlockRange = { pid, from: from - start, to: end - start };
    const tokens = (tokensByPid.get(pid) ?? []).filter(
      (token) => !token.is_punct && token.span > 0
    );
    const token = tokenForSelection(tokens, from - start, end - start);
    if (!token) return;
    const key = `${pid}:${token.index}:${token.surface}`;
    if (key === lastTextSelectionKey) return;
    lastTextSelectionKey = key;
    onSelectToken(pid, token);
  }

  const selectionSyncPlugin = new Plugin({
    props: {
      handleDOMEvents: {
        mouseup: (current) => {
          // Dopo il click ProseMirror ha già aggiornato la selezione.
          window.setTimeout(() => selectionFromView(current), 0);
          return false;
        },
        keyup: (current, event) => {
          if (
            [
              "ArrowLeft",
              "ArrowRight",
              "ArrowUp",
              "ArrowDown",
              "Home",
              "End",
              "PageUp",
              "PageDown",
            ].includes(event.key)
          ) {
            selectionFromView(current);
          }
          return false;
        },
      },
    },
  });

  function ensurePids(state: EditorState): EditorState {
    const targets: { pos: number; attrs: Record<string, unknown> }[] = [];
    state.doc.descendants((node, pos) => {
      if (
        (node.type.name === "paragraph" || node.type.name === "heading") &&
        !node.attrs.pid
      ) {
        targets.push({ pos, attrs: { ...node.attrs, pid: newPid() } });
      }
    });
    if (targets.length === 0) return state;
    const tr = state.tr;
    for (const target of targets) tr.setNodeMarkup(target.pos, null, target.attrs);
    return state.apply(tr);
  }

  function initialDoc() {
    return schema.node("doc", null, [
      schema.node("paragraph", { pid: newPid() }, []),
    ]);
  }

  function asRanges(value: unknown): { start: number; end: number }[] {
    if (!Array.isArray(value)) return [];
    return value
      .filter((item) => item && typeof item === "object")
      .map((item) => ({
        start: Number((item as { start?: unknown }).start ?? 0),
        end: Number((item as { end?: unknown }).end ?? 0),
      }))
      .filter((range) => range.start < range.end);
  }

  function collectParagraphs(): ParagraphInfo[] {
    if (!view) return [];
    const out: ParagraphInfo[] = [];
    view.state.doc.descendants((node) => {
      if (node.type.name === "paragraph" || node.type.name === "heading") {
        out.push({
          pid: String(node.attrs.pid ?? "detached"),
          text: node.textContent,
          style: node.type.name === "heading" ? "heading" : "paragraph",
          merges: asRanges(node.attrs.caa_merges),
          splits: asRanges(node.attrs.caa_splits),
        });
      }
    });
    return out;
  }

  function options(paragraph: ParagraphInfo) {
    return {
      intensity,
      mode,
      document_key: documentKey,
      commercial_only: commercialOnly,
      pack_id: packId,
      paragraph_id: paragraph.pid,
      symbolize_articles: symbolizeArticles,
      symbolize_prepositions: symbolizePrepositions,
      symbolize_conjunctions: symbolizeConjunctions,
      manual_units: paragraph.merges,
      split_ranges: paragraph.splits,
    };
  }

  async function analyzeAll() {
    if (!view) return;
    if (analyzing) {
      queued = true;
      return;
    }
    analyzing = true;
    try {
      const paragraphs = collectParagraphs();
      if (!autoEnabled) {
        tokensByPid.clear();
        refreshStrips();
        return;
      }
      let budget = 60;
      for (const paragraph of paragraphs) {
        if (budget <= 0) {
          queued = true;
          break;
        }
        if (!paragraph.text.trim()) {
          analyzedSignature.set(paragraph.pid, "");
          tokensByPid.set(paragraph.pid, []);
          continue;
        }
        const signature = signatureFor(paragraph);
        if (analyzedSignature.get(paragraph.pid) === signature) continue;
        budget -= 1;
        try {
          const result = isTauri()
            ? await api.symbolizeParagraph(paragraph.text, options(paragraph))
            : demoAnalyze(paragraph.text, paragraph.pid);
          analyzedSignature.set(paragraph.pid, signature);
          tokensByPid.set(paragraph.pid, result.tokens);
        } catch (error) {
          analyzedSignature.set(paragraph.pid, signature);
          const message = error instanceof Error ? error.message : String(error);
          if (!message.includes("Anteprima browser")) onError(message);
        }
      }
      onAnalyzed(new Map(tokensByPid));
      refreshStrips();
      scheduleRefine();
    } finally {
      analyzing = false;
      if (queued) {
        queued = false;
        schedule(80);
      }
    }
  }

  function schedule(delay = 240) {
    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      debounce = null;
      void analyzeAll();
    }, delay);
  }

  /// Passata neurale facoltativa: parte dopo una breve pausa di scrittura, senza
  /// bloccare la digitazione. Ogni nuova analisi annulla la passata precedente; il
  /// risultato si applica solo se il testo non è cambiato.
  function scheduleRefine(delay = 450) {
    if (refineTimer) clearTimeout(refineTimer);
    refineTimer = setTimeout(() => {
      refineTimer = null;
      void refineAll();
    }, delay);
  }

  async function refineAll() {
    if (!autoEnabled || !neural || !isTauri() || !view) return;
    const seq = ++refineSeq;
    for (const paragraph of collectParagraphs()) {
      if (seq !== refineSeq) return;
      const text = paragraph.text.trim();
      // Basta un testo con almeno tre parole: aspettare il punto finale
      // significava non vedere mai la correzione contestuale mentre si scrive.
      if (!text || text.split(/\s+/).filter(Boolean).length < 3) continue;
      const signature = signatureFor(paragraph);
      if (refinedSignature.get(paragraph.pid) === signature) continue;
      try {
        const result = await api.refineParagraph(paragraph.text, options(paragraph));
        if (seq !== refineSeq) return;
        const current = collectParagraphs().find((item) => item.pid === paragraph.pid);
        if (!current || signatureFor(current) !== signature) continue;
        refinedSignature.set(paragraph.pid, signature);
        tokensByPid.set(paragraph.pid, result.tokens);
        refreshStrips();
        onAnalyzed(new Map(tokensByPid));
      } catch {
        // Modello assente o in errore: resta valida la scelta lessicale, senza avvisi.
        refinedSignature.set(paragraph.pid, signature);
      }
    }
  }

  /// Invio: nuova voce negli elenchi, nuovo paragrafo altrove. Non invia
  /// prompt né conferma suggerimenti: il cursore va semplicemente a capo.
  function splitBlockOrList(state: EditorState, dispatch?: (tr: Transaction) => void) {
    if (splitListItem(schema.nodes.list_item)(state, dispatch)) return true;
    return splitBlock(state, dispatch);
  }

  /// Plugin comuni a tutti gli stati dell'editor: caricare un documento o
  /// importare testo non deve perdere scorciatoie e sincronizzazione selezione.
  function editorPlugins() {
    return [
      history(),
      selectionSyncPlugin,
      keymap({
        "Mod-z": undo,
        "Mod-y": redo,
        "Shift-Mod-z": redo,
        "Mod-b": toggleMark(schema.marks.strong),
        "Mod-i": toggleMark(schema.marks.em),
        "Mod-u": toggleMark(schema.marks.underline),
        "Mod-Shift-x": toggleMark(schema.marks.strike),
        "Mod-a": selectAll,
        Enter: splitBlockOrList,
        Tab: sinkListItem(schema.nodes.list_item),
        "Shift-Tab": liftListItem(schema.nodes.list_item),
      }),
      keymap(baseKeymap),
    ];
  }

  onMount(() => {
    let state = EditorState.create({
      doc: initialDoc(),
      plugins: editorPlugins(),
    });
    state = ensurePids(state);

    view = new EditorView(host, {
      state,
      nodeViews: {
        paragraph: textViewFor("paragraph"),
        heading: textViewFor("heading"),
      },
      dispatchTransaction(transaction) {
        const next = view!.state.apply(transaction);
        view!.updateState(next);
        if (transaction.docChanged) {
          const withIds = ensurePids(view!.state);
          if (withIds !== view!.state) view!.updateState(withIds);
          onDocChanged?.();
          schedule();
        }
        reportFormatting();
      },
      attributes: {
        class: "ProseMirror editor",
        role: "textbox",
        "aria-multiline": "true",
        "aria-label": "Documento CAA",
        spellcheck: "true",
      },
    });

    reportFormatting();
    schedule(30);
    return () => {
      view?.destroy();
      view = null;
      paragraphViews.clear();
    };
  });

  // Cambio di modalità/intensità/preferenze: rivaluta i paragrafi.
  $effect(() => {
    void mode;
    void intensity;
    void commercialOnly;
    void packId;
    void documentKey;
    if (!view) return;
    analyzedSignature.clear();
    refinedSignature.clear();
    refineSeq += 1;
    schedule(60);
  });

  // Simbolizzazione spenta o modalità solo testo: nasconde le strisce senza
  // perdere le associazioni già calcolate.
  $effect(() => {
    void autoEnabled;
    void mode;
    if (!view) return;
    refreshStrips();
  });

  // Evidenzia il pittogramma selezionato ricostruendo i chip.
  $effect(() => {
    void selectedUnit;
    if (!view) return;
    refreshStrips();
  });

  export function getDocJson(): unknown {
    return view ? view.state.doc.toJSON() : null;
  }

  export function setDocJson(json: unknown, plain?: string) {
    if (!view) return;
    let doc = null;
    try {
      doc = json ? schema.nodeFromJSON(json) : null;
    } catch {
      doc = null;
    }
    if (!doc) doc = schema.node("doc", null, [schema.node("paragraph", { pid: newPid() }, [])]);
    const state = ensurePids(
      EditorState.create({ doc, plugins: editorPlugins() })
    );
    paragraphViews.clear();
    view.updateState(state);
    analyzedSignature.clear();
    refinedSignature.clear();
    tokensByPid.clear();
    void plain;
    reportFormatting();
    schedule(40);
  }

  export function insertPlainText(text: string) {
    if (!view) return;
    const paragraphs = text
      .replace(/\r\n/g, "\n")
      .split(/\n{1,}/)
      .map((line) => line.trim())
      .filter((line, index, all) => line.length > 0 || all.length === 1)
      .map((line) =>
        schema.node(
          "paragraph",
          { pid: newPid() },
          line ? [schema.text(line)] : []
        )
      );
    const doc = schema.node("doc", null, paragraphs.length ? paragraphs : [schema.node("paragraph", { pid: newPid() }, [])]);
    paragraphViews.clear();
    view.updateState(EditorState.create({ doc, plugins: editorPlugins() }));
    analyzedSignature.clear();
    refinedSignature.clear();
    tokensByPid.clear();
    reportFormatting();
    schedule(40);
    view.focus();
  }

  export function getPlainText(): string {
    return collectParagraphs()
      .map((paragraph) => paragraph.text)
      .join("\n\n");
  }

  export function getParagraphs() {
    return collectParagraphs();
  }

  export function getTokens(): Map<string, CaaToken[]> {
    return new Map(tokensByPid);
  }

  export function refreshAll() {
    analyzedSignature.clear();
    refinedSignature.clear();
    schedule(0);
  }

  /// Applica subito un pittogramma scelto nel pannello, senza aspettare la
  /// rianalisi: il foglio si aggiorna nello stesso istante in cui l'utente
  /// sceglie il suggerimento. Il testo non viene toccato.
  /// Token corrente per indice: serve a verificare che la scelta del pannello
  /// non venga applicata a una parola diversa dopo una rianalisi.
  export function tokenAt(pid: string, index: number): CaaToken | null {
    return tokensByPid.get(pid)?.find((token) => token.index === index) ?? null;
  }

  export function applyTokenSymbol(
    pid: string,
    index: number,
    alternative: {
      symbol_id: string;
      svg: string | null;
      label: string;
      meaning: string;
      concept_id: string | null;
      approximate: boolean;
    }
  ): boolean {
    const tokens = tokensByPid.get(pid);
    if (!tokens) return false;
    const current = tokens.find((token) => token.index === index);
    if (!current) return false;
    const updated: CaaToken = {
      ...current,
      symbol_id: alternative.symbol_id,
      svg: alternative.svg,
      // La didascalia visibile resta la parola scritta; l'etichetta del
      // repertorio va nei metadati, non sotto il pittogramma.
      caption: current.surface,
      symbol_label: alternative.label,
      concept_id: alternative.concept_id,
      concept_label: alternative.label,
      meaning: alternative.meaning,
      strategy: "manual",
      approximate: alternative.approximate,
      locked: true,
    };
    tokensByPid.set(
      pid,
      tokens.map((token) => (token.index === index ? updated : token))
    );
    // La firma non corrisponde più al risultato mostrato: la prossima analisi
    // deve rileggere il paragrafo dal motore.
    analyzedSignature.delete(pid);
    flashApplied(pid, index);
    refreshStrips();
    onAnalyzed(new Map(tokensByPid));
    onSelectToken(pid, updated);
    return true;
  }

  /// Applica lo stesso pittogramma a più unità in un solo aggiornamento:
  /// usato per l'ambito «tutte le occorrenze nel documento». Restituisce
  /// quante unità sono state aggiornate.
  export function applyTokenSymbols(
    entries: {
      pid: string;
      index: number;
      alternative: {
        symbol_id: string;
        svg: string | null;
        label: string;
        meaning: string;
        concept_id: string | null;
        approximate: boolean;
      };
    }[]
  ): number {
    let count = 0;
    let selectedUpdate: { pid: string; token: CaaToken } | null = null;
    for (const entry of entries) {
      const tokens = tokensByPid.get(entry.pid);
      if (!tokens) continue;
      const current = tokens.find((token) => token.index === entry.index);
      if (!current) continue;
      const updated: CaaToken = {
        ...current,
        symbol_id: entry.alternative.symbol_id,
        svg: entry.alternative.svg,
        caption: current.surface,
        symbol_label: entry.alternative.label,
        concept_id: entry.alternative.concept_id ?? current.concept_id,
        concept_label: entry.alternative.label,
        meaning: entry.alternative.meaning,
        strategy: "manual",
        approximate: entry.alternative.approximate,
        locked: true,
      };
      tokensByPid.set(
        entry.pid,
        tokens.map((token) => (token.index === entry.index ? updated : token))
      );
      analyzedSignature.delete(entry.pid);
      selectedUpdate = { pid: entry.pid, token: updated };
      count += 1;
    }
    if (count > 0) {
      if (selectedUpdate) flashApplied(selectedUpdate.pid, selectedUpdate.token.index);
      refreshStrips();
      onAnalyzed(new Map(tokensByPid));
      if (selectedUpdate) onSelectToken(selectedUpdate.pid, selectedUpdate.token);
    }
    return count;
  }

  /// Aggiorna sul posto blocco, visibilità e modificatori dell'unità: il foglio
  /// cambia subito, senza attendere una rianalisi.
  export function applyTokenState(
    pid: string,
    index: number,
    patch: { locked?: boolean; hidden?: boolean; modifiers?: CaaToken["modifiers"] }
  ) {
    const tokens = tokensByPid.get(pid);
    if (!tokens) return;
    const current = tokens.find((token) => token.index === index);
    if (!current) return;
    const updated: CaaToken = {
      ...current,
      locked: patch.locked ?? current.locked,
      hidden: patch.hidden ?? current.hidden,
      modifiers: patch.modifiers ?? current.modifiers,
    };
    tokensByPid.set(
      pid,
      tokens.map((token) => (token.index === index ? updated : token))
    );
    refreshStrips();
    onAnalyzed(new Map(tokensByPid));
    onSelectToken(pid, updated);
  }

  export function refreshParagraph(pid: string) {
    analyzedSignature.delete(pid);
    refinedSignature.delete(pid);
    schedule(0);
  }

  function blockForPid(pid: string): { pos: number; node: PMNode } | null {
    if (!view) return null;
    let found: { pos: number; node: PMNode } | null = null;
    view.state.doc.descendants((node, pos) => {
      if (
        !found &&
        (node.type.name === "paragraph" || node.type.name === "heading") &&
        String(node.attrs.pid ?? "") === pid
      ) {
        found = { pos, node };
        return false;
      }
    });
    return found;
  }

  function mergeRange(
    list: { start: number; end: number }[],
    range: { start: number; end: number }
  ): { start: number; end: number }[] {
    const sorted = [...list, range].sort((a, b) => a.start - b.start);
    const out: { start: number; end: number }[] = [];
    for (const item of sorted) {
      const last = out[out.length - 1];
      if (last && item.start <= last.end + 1) {
        last.end = Math.max(last.end, item.end);
      } else {
        out.push({ ...item });
      }
    }
    return out;
  }

  function overlaps(
    a: { start: number; end: number },
    b: { start: number; end: number }
  ): boolean {
    return a.start < b.end && a.end > b.start;
  }

  function setBlockRanges(
    pid: string,
    merges: { start: number; end: number }[],
    splits: { start: number; end: number }[]
  ): boolean {
    const block = blockForPid(pid);
    if (!view || !block) return false;
    const tr = view.state.tr.setNodeMarkup(block.pos, null, {
      ...block.node.attrs,
      caa_merges: merges,
      caa_splits: splits,
    });
    view.dispatch(tr);
    analyzedSignature.delete(pid);
    refinedSignature.delete(pid);
    schedule(0);
    return true;
  }

  /// Unisce in una sola unità CAA le parole della selezione corrente (o
  /// dell'ultima selezione non vuota). L'operazione è una transazione
  /// ProseMirror: entra nella cronologia e viene salvata nel documento.
  export function mergeSelection(): boolean {
    if (!view) return false;
    const selection = view.state.selection;
    let from = selection.from;
    let to = selection.to;
    const resolved = view.state.doc.resolve(from);
    let depth = resolved.depth;
    while (
      depth > 0 &&
      resolved.node(depth).type.name !== "paragraph" &&
      resolved.node(depth).type.name !== "heading"
    ) {
      depth -= 1;
    }
    if (depth === 0) return false;
    const block = resolved.node(depth);
    const pid = String(block.attrs.pid ?? "");
    if (!pid) return false;
    const start = resolved.start(depth);
    let relFrom = from - start;
    let relTo = Math.min(to, resolved.end(depth)) - start;
    if (relFrom === relTo && lastBlockRange?.pid === pid) {
      relFrom = lastBlockRange.from;
      relTo = lastBlockRange.to;
    }
    if (relTo <= relFrom) return false;

    const tokens = (tokensByPid.get(pid) ?? []).filter(
      (token) => !token.is_punct && token.span > 0
    );
    const covered = tokens.filter((token) => token.end > relFrom && token.start < relTo);
    if (covered.length < 2) return false;
    const range = {
      start: Math.min(...covered.map((token) => token.start)),
      end: Math.max(...covered.map((token) => token.end)),
    };
    const paragraph = collectParagraphs().find((item) => item.pid === pid);
    if (!paragraph) return false;
    const merges = mergeRange(paragraph.merges, range);
    const splits = paragraph.splits.filter((item) => !overlaps(item, range));
    return setBlockRanges(pid, merges, splits);
  }

  /// Separa l'unità indicata: le parole tornano unità distinte e il motore
  /// non le rifonde automaticamente in questa occorrenza.
  export function splitUnit(pid: string, index: number): boolean {
    const token = (tokensByPid.get(pid) ?? []).find((item) => item.index === index);
    if (!token || token.span < 2) return false;
    const range = { start: token.start, end: token.end };
    const paragraph = collectParagraphs().find((item) => item.pid === pid);
    if (!paragraph) return false;
    const merges = paragraph.merges.filter((item) => !overlaps(item, range));
    const splits = mergeRange(paragraph.splits, range);
    return setBlockRanges(pid, merges, splits);
  }

  export function focus() {
    view?.focus();
  }

  export function toggleBold() {
    if (!view) return;
    toggleMark(schema.marks.strong)(view.state, view.dispatch);
    view.focus();
  }

  export function toggleItalic() {
    if (!view) return;
    toggleMark(schema.marks.em)(view.state, view.dispatch);
    view.focus();
  }

  export function toggleUnderline() {
    if (!view) return;
    toggleMark(schema.marks.underline)(view.state, view.dispatch);
    view.focus();
  }

  export function toggleStrike() {
    if (!view) return;
    toggleMark(schema.marks.strike)(view.state, view.dispatch);
    view.focus();
  }

  export function toggleHighlight() {
    if (!view) return;
    toggleMark(schema.marks.highlight, { color: "#fff59d" })(view.state, view.dispatch);
    view.focus();
  }

  export function clearFormatting() {
    if (!view) return;
    const { from, to } = view.state.selection;
    view.dispatch(view.state.tr.removeMark(from, to));
    view.focus();
  }

  export function selectAllText() {
    if (!view) return;
    selectAll(view.state, view.dispatch);
    view.focus();
  }

  export function getSelectedText(): string {
    if (!view) return "";
    const { from, to } = view.state.selection;
    return view.state.doc.textBetween(from, to, "\n");
  }

  /// Testo dall'inizio del blocco al cursore, usato dalla lettura durante la
  /// digitazione. Non modifica il documento.
  export function textBeforeCursor(): string {
    if (!view) return "";
    const { from } = view.state.selection;
    const start = view.state.selection.$from.start();
    return view.state.doc.textBetween(start, from, " ");
  }

  export function currentBlockText(): string {
    if (!view) return "";
    return view.state.selection.$from.parent.textContent;
  }

  export function wordBeforeCursor(): string {
    const text = textBeforeCursor();
    const match = text.match(/[\p{L}\p{N}][\p{L}\p{N}'’\-]*\s*$/u);
    return match ? match[0].trim() : "";
  }

  /// Ultima frase già chiusa da punteggiatura prima del cursore. Da chiamare
  /// dopo che il carattere è stato inserito nel documento.
  export function sentenceBeforeCursor(): string {
    return completedSentenceBefore(textBeforeCursor());
  }

  /// Ultima frase del blocco corrente, utile su Invio prima che la divisione
  /// del paragrafo cambi il contenuto.
  export function currentSentenceText(): string {
    return lastSentence(currentBlockText());
  }

  export function deleteSelection() {
    if (!view || view.state.selection.empty) return;
    view.dispatch(view.state.tr.deleteSelection());
    view.focus();
  }

  export function insertTextAtSelection(text: string) {
    if (!view || !text) return;
    view.dispatch(view.state.tr.insertText(text));
    view.focus();
  }

  export function insertPageBreak() {
    if (!view) return;
    const node = schema.nodes.paragraph.create({ pid: newPid(), page_break: true });
    view.dispatch(view.state.tr.replaceSelectionWith(node));
    view.focus();
  }

  export function applyParagraphAttr(patch: Record<string, unknown>) {
    if (!view) return;
    const { from, to } = view.state.selection;
    const tr = view.state.tr;
    view.state.doc.nodesBetween(from, to, (node, pos) => {
      if (node.type.name === "paragraph" || node.type.name === "heading") {
        tr.setNodeMarkup(pos, null, { ...node.attrs, ...patch });
      }
    });
    if (tr.docChanged) view.dispatch(tr);
    view.focus();
  }

  export function adjustFontSize(delta: number) {
    if (!view) return;
    const state = view.state;
    const marks = state.storedMarks ?? state.selection.$from.marks();
    const style = marks.find((mark) => mark.type.name === "text_style");
    const current = Number.parseFloat(style?.attrs.font_size ?? "15") || 15;
    const size = Math.min(48, Math.max(8, Math.round(current + delta)));
    const mark = schema.marks.text_style.create({
      color: style?.attrs.color ?? null,
      font_size: `${size}px`,
    });
    const { from, to, empty } = state.selection;
    const tr = state.tr;
    if (empty) {
      tr.addStoredMark(mark);
    } else {
      tr.removeMark(from, to, schema.marks.text_style).addMark(from, to, mark);
    }
    view.dispatch(tr);
    view.focus();
  }

  let copiedMarks: Mark[] = [];

  export function copyFormatting() {
    if (!view) return;
    const marks = view.state.storedMarks ?? view.state.selection.$from.marks();
    copiedMarks = marks.map((mark) => mark.type.create(mark.attrs));
  }

  export function applyCopiedFormatting() {
    if (!view || copiedMarks.length === 0) return;
    const { from, to, empty } = view.state.selection;
    const tr = view.state.tr;
    if (empty) {
      tr.setStoredMarks(copiedMarks.map((mark) => mark.type.create(mark.attrs)));
    } else {
      for (const mark of copiedMarks) {
        tr.addMark(from, to, mark.type.create(mark.attrs));
      }
    }
    view.dispatch(tr);
    view.focus();
  }

  function textMatches(query: string): { from: number; to: number }[] {
    if (!view || !query) return [];
    const matches: { from: number; to: number }[] = [];
    view.state.doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;
      let index = 0;
      while (index <= node.text.length) {
        const found = node.text.indexOf(query, index);
        if (found < 0) break;
        matches.push({ from: pos + found, to: pos + found + query.length });
        index = found + Math.max(1, query.length);
      }
    });
    return matches;
  }

  export function countMatches(query: string): number {
    return textMatches(query).length;
  }

  export function findNext(query: string, forward = true): boolean {
    if (!view || !query) return false;
    const matches = textMatches(query);
    if (matches.length === 0) return false;
    const anchor = view.state.selection.from;
    let match = forward
      ? matches.find((item) => item.from >= anchor)
      : [...matches].reverse().find((item) => item.to <= anchor);
    if (!match) match = forward ? matches[0] : matches[matches.length - 1];
    const tr = view.state.tr
      .setSelection(TextSelection.create(view.state.doc, match.from, match.to))
      .scrollIntoView();
    view.dispatch(tr);
    view.focus();
    return true;
  }

  export function replaceCurrent(query: string, replacement: string): boolean {
    if (!view || !query) return false;
    const { from, to } = view.state.selection;
    if (view.state.doc.textBetween(from, to) !== query) {
      if (!findNext(query)) return false;
    }
    if (view.state.doc.textBetween(view.state.selection.from, view.state.selection.to) !== query) {
      return false;
    }
    view.dispatch(view.state.tr.insertText(replacement));
    findNext(query);
    return true;
  }

  export function replaceAll(query: string, replacement: string): number {
    if (!view || !query) return 0;
    const matches = textMatches(query);
    if (matches.length === 0) return 0;
    const tr = view.state.tr;
    for (const match of [...matches].reverse()) {
      tr.insertText(replacement, match.from, match.to);
    }
    view.dispatch(tr);
    view.focus();
    return matches.length;
  }

  export function selectionInTable(): boolean {
    return view ? isInTable(view.state) : false;
  }

  export function tableAction(
    action: "addRowAfter" | "addColumnAfter" | "deleteRow" | "deleteColumn" | "mergeCells" | "splitCell"
  ): boolean {
    if (!view) return false;
    const commands = {
      addRowAfter,
      addColumnAfter,
      deleteRow,
      deleteColumn,
      mergeCells,
      splitCell,
    };
    const executed = commands[action](view.state, view.dispatch);
    if (executed) view.focus();
    return executed;
  }

  export function applyTextStyle(color: string | null, fontSize: string | null) {
    if (!view) return;
    const { from, to, empty } = view.state.selection;
    if (empty) return;
    const mark = schema.marks.text_style.create({ color, font_size: fontSize });
    const tr = view.state.tr
      .removeMark(from, to, schema.marks.text_style)
      .addMark(from, to, mark);
    view.dispatch(tr);
    view.focus();
  }

  export function applyAlignment(align: string | null) {
    if (!view) return;
    const tr = view.state.tr;
    const { from, to } = view.state.selection;
    view.state.doc.nodesBetween(from, to, (node, pos) => {
      if (node.type.name === "paragraph" || node.type.name === "heading") {
        tr.setNodeMarkup(pos, null, { ...node.attrs, align });
      }
    });
    view.dispatch(tr);
    view.focus();
  }

  export function applyHeading(level: number | null) {
    if (!view) return;
    const type = level ? schema.nodes.heading : schema.nodes.paragraph;
    setBlockType(type, level ? { level } : {})(view.state, view.dispatch);
    view.focus();
  }

  export function toggleList(kind: "bullet" | "ordered") {
    if (!view) return;
    const type = kind === "bullet" ? schema.nodes.bullet_list : schema.nodes.ordered_list;
    wrapInList(type)(view.state, view.dispatch);
    view.focus();
  }

  export function insertTable(rows = 2, columns = 2) {
    if (!view) return;
    const cell = schema.nodes.table_cell;
    const row = schema.nodes.table_row;
    const tableType = schema.nodes.table;
    const rowNodes: import("prosemirror-model").Node[] = [];
    for (let r = 0; r < rows; r += 1) {
      const cells: import("prosemirror-model").Node[] = [];
      for (let c = 0; c < columns; c += 1) {
        cells.push(cell.createAndFill() as never);
      }
      rowNodes.push(row.create(null, cells));
    }
    const node = tableType.create(null, rowNodes);
    view.dispatch(view.state.tr.replaceSelectionWith(node));
    view.focus();
  }

  export function undoAction() {
    if (!view) return;
    undo(view.state, view.dispatch);
  }

  export function redoAction() {
    if (!view) return;
    redo(view.state, view.dispatch);
  }
</script>

<div
  class="editor"
  bind:this={host}
  data-mode={mode}
  data-captions={showCaptions ? "true" : "false"}
  data-border={borderStyle}
></div>
