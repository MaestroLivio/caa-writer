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
  import { isExplicitContent } from "../content-filter";
  import type { CaaMode, CaaToken, Intensity } from "../types";

  interface Props {
    mode: CaaMode;
    intensity: Intensity;
    documentKey: string;
    commercialOnly: boolean;
    /// Bordo del pittogramma (nessuno, immagine, immagine+testo, separato, divisore).
    borderStyle: string;
    packId: string | null;
    selectedUnit: { pid: string; index: number } | null;
    /// Simbolizzazione automatica: se disattivata i pittogrammi restano memorizzati
    /// ma non vengono ricalcolati né mostrati.
    autoEnabled: boolean;
    /// Reranking contestuale con il modello locale (già verificato come disponibile).
    neural: boolean;
    /// Inserisce automaticamente i pittogrammi ARASAAC per le parole fuori
    /// lessico, dopo l'analisi e senza bloccare la digitazione.
    onlineAutoInsert?: boolean;
    /// Simbolizzazione delle parole grammaticali, per categoria.
    symbolizeArticles: boolean;
    symbolizePrepositions: boolean;
    symbolizeConjunctions: boolean;
    /// Se falso, i simboli applicati a mano non vengono mostrati (restano
    /// salvati nel documento): utile per la stampa o per ridurre il carico
    /// visivo senza perdere le scelte.
    showExplicitSymbols?: boolean;
    /// `riga` = striscia unica sopra il paragrafo; `parola` = ogni pittogramma
    /// sopra (o sotto) la parola a cui si riferisce.
    symbolLayout?: "riga" | "parola";
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
    /// Attesa prima dell'analisi lessicale (profilo prestazionale).
    analysisDebounceMs?: number;
    /// Attesa prima della rifinitura neurale.
    refineDebounceMs?: number;
    /// Profilo attivo: le preferenze di un profilo non si applicano agli altri.
    profileId?: string | null;
    /// Indicatore non invasivo «Analisi in corso».
    onAnalysisState?: (active: boolean) => void;
    /// Misure reali raccolte nel WebView per il pannello Diagnostica.
    onMetrics?: (metrics: {
      input_samples: number;
      input_p95_ms: number;
      input_worst_ms: number;
      update_p95_ms: number;
      analysis_runs: number;
      long_tasks: number;
      longest_task_ms: number;
      last_analysis_ms: number;
    }) => void;
  }

  let {
    mode,
    intensity,
    documentKey,
    commercialOnly,
    borderStyle,
    packId,
    selectedUnit,
    autoEnabled,
    neural,
    onlineAutoInsert = false,
    symbolizeArticles,
    symbolizePrepositions,
    symbolizeConjunctions,
    showExplicitSymbols = true,
    symbolLayout = "riga",
    onSelectToken,
    onTokenMenu,
    onError,
    onAnalyzed,
    onFormattingChange,
    onDocChanged,
    analysisDebounceMs = 240,
    refineDebounceMs = 600,
    profileId = null,
    onAnalysisState,
    onMetrics,
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
  let refineRequestId = "";
  const refinedSignature = new Map<string, string>();
  let analyzeSeq = 0;
  let onlineBusy = false;
  /// Parole già richieste alle API in questa sessione: evita richieste
  /// ripetute e il ciclo analisi → rete → analisi.
  const onlineAttempted = new Set<string>();

  // Misure raccolte nel WebView: sono tempi reali, non stime. Gli array
  // contengono gli ultimi 200 campioni, sufficienti per un p95 significativo
  // senza far crescere la memoria durante una sessione lunga.
  const inputSamples: number[] = [];
  const updateSamples: number[] = [];
  let lastKeyAt = 0;
  let analysisRuns = 0;
  let lastAnalysisMs = 0;
  let longTasks = 0;
  let longestTaskMs = 0;

  function percentile(values: number[], ratio: number): number {
    if (values.length === 0) return 0;
    const sorted = [...values].sort((a, b) => a - b);
    const index = Math.min(sorted.length - 1, Math.floor(sorted.length * ratio));
    return Math.round(sorted[index] * 10) / 10;
  }

  function reportMetrics() {
    onMetrics?.({
      input_samples: inputSamples.length,
      input_p95_ms: percentile(inputSamples, 0.95),
      input_worst_ms: inputSamples.length ? Math.round(Math.max(...inputSamples) * 10) / 10 : 0,
      update_p95_ms: percentile(updateSamples, 0.95),
      analysis_runs: analysisRuns,
      long_tasks: longTasks,
      longest_task_ms: Math.round(longestTaskMs * 10) / 10,
      last_analysis_ms: Math.round(lastAnalysisMs * 10) / 10,
    });
  }

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
  /// Range DOM corrispondente agli offset (UTF-16) di un token nel paragrafo.
  /// ProseMirror gestisce gli stessi offset, quindi non serve modificare il
  /// documento per allineare i pittogrammi alle parole.
  function rangeForOffsets(root: HTMLElement, start: number, end: number): Range | null {
    if (end <= start) return null;
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    let cursor = 0;
    let startNode: Text | null = null;
    let endNode: Text | null = null;
    let startOffset = 0;
    let endOffset = 0;
    let node = walker.nextNode() as Text | null;
    while (node) {
      const length = node.data.length;
      if (!startNode && cursor + length >= start) {
        startNode = node;
        startOffset = Math.max(0, start - cursor);
      }
      if (!endNode && cursor + length >= end) {
        endNode = node;
        endOffset = Math.max(0, end - cursor);
        break;
      }
      cursor += length;
      node = walker.nextNode() as Text | null;
    }
    if (!startNode || !endNode) return null;
    const range = document.createRange();
    range.setStart(startNode, Math.min(startOffset, startNode.data.length));
    range.setEnd(endNode, Math.min(endOffset, endNode.data.length));
    return range;
  }

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

  // Cache dei nodi SVG già analizzati: il documento contiene molti pittogrammi
  // ripetuti e ricostruire l'HTML a ogni aggiornamento faceva riparsare gli
  // stessi SVG. La cache è limitata (LRU) per non trattenere migliaia di nodi.
  const svgPrototypes = new Map<string, Element>();
  const SVG_CACHE_LIMIT = 300;

  function svgNode(markup: string): Element | null {
    const cached = svgPrototypes.get(markup);
    if (cached) {
      // Rinnova l'ordine LRU.
      svgPrototypes.delete(markup);
      svgPrototypes.set(markup, cached);
      return cached.cloneNode(true) as Element;
    }
    const template = document.createElement("template");
    template.innerHTML = markup;
    const node = template.content.firstElementChild;
    if (!node) return null;
    if (svgPrototypes.size >= SVG_CACHE_LIMIT) {
      const oldest = svgPrototypes.keys().next().value;
      if (oldest) svgPrototypes.delete(oldest);
    }
    svgPrototypes.set(markup, node);
    return node.cloneNode(true) as Element;
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
      if (token.image) {
        // ARASAAC e pittogrammi importati: PNG usato direttamente.
        const img = document.createElement("img");
        img.src = token.image;
        img.alt = "";
        img.draggable = false;
        media.append(img);
      } else if (token.svg && token.svg.trimStart().startsWith("<svg")) {
        const node = svgNode(token.svg);
        if (node) media.append(node);
      } else if (token.composition.length > 0) {
        media.classList.add("caa-chip__media--composition");
        for (const part of token.composition) {
          const partSpan = document.createElement("span");
          if (part.image) {
            const img = document.createElement("img");
            img.src = part.image;
            img.alt = part.label;
            img.draggable = false;
            partSpan.append(img);
          } else if (part.svg && part.svg.trimStart().startsWith("<svg")) {
            const node = svgNode(part.svg);
            if (node) partSpan.append(node);
          }
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
      // Il chip sta sopra il testo ma dentro il blocco ProseMirror: senza
      // stopPropagation il clic può essere riassorbito dall'editor e la
      // selezione non arriva al pannello (su Windows era sistematico).
      button.addEventListener("pointerdown", (event) => event.stopPropagation());
      button.addEventListener("mousedown", (event) => {
        event.preventDefault();
        event.stopPropagation();
      });
      button.addEventListener("click", (event) => {
        event.preventDefault();
        event.stopPropagation();
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
    /// Firma dell'ultimo contenuto disegnato: evita di ricostruire i chip a
    /// ogni transazione ProseMirror (cioè a ogni tasto) quando i token non
    /// sono cambiati.
    lastKey = "";
    private positionFrame: number | null = null;
    private onResize = () => this.positionChips();

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
      window.addEventListener("resize", this.onResize);
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

    private symbolSpace(): number {
      const raw = getComputedStyle(this.dom).getPropertyValue("--symbol-size");
      const size = Number.parseFloat(raw) || 56;
      return Math.min(104, Math.max(32, size)) + 6;
    }

    /// Con la disposizione «sulla parola» la striscia diventa un livello
    /// assoluto: ogni chip viene centrato sul range di testo della propria
    /// unità. Il testo resta nel contentDOM, quindi ProseMirror non perde
    /// caratteri né assorbe didascalie.
    private applyOnWordLayout(active: boolean) {
      const below = mode === "sotto";
      this.dom.classList.toggle("caa-block--on-word", active && !below);
      this.dom.classList.toggle("caa-block--on-word-below", active && below);
      if (!active) {
        this.dom.style.paddingTop = "";
        this.dom.style.paddingBottom = "";
        this.contentDOM.style.lineHeight = (this.node.attrs.line_height as string | null) ?? "";
        return;
      }
      const space = this.symbolSpace();
      if (below) {
        this.dom.style.paddingTop = "";
        this.dom.style.paddingBottom = `${space}px`;
      } else {
        this.dom.style.paddingTop = `${space}px`;
        this.dom.style.paddingBottom = "";
      }
      // Le righe successive hanno bisogno di spazio per i pittogrammi della
      // riga precedente: si aumenta solo in questa disposizione.
      const current = Number.parseFloat(getComputedStyle(this.contentDOM).lineHeight) || 0;
      this.contentDOM.style.lineHeight = `${Math.max(current, space + 8)}px`;
    }

    positionChips() {
      if (this.positionFrame !== null) cancelAnimationFrame(this.positionFrame);
      this.positionFrame = requestAnimationFrame(() => {
        this.positionFrame = null;
        const active = symbolLayout === "parola" && !this.strip.hidden && mode !== "testo";
        this.applyOnWordLayout(active);
        if (!active) return;
        const rootRect = this.dom.getBoundingClientRect();
        const space = this.symbolSpace();
        const tokens = tokensByPid.get(this.pid) ?? [];
        const chips = this.strip.querySelectorAll<HTMLElement>(".caa-chip");
        chips.forEach((chip) => {
          const index = Number(chip.dataset.index);
          const token = tokens.find((item) => item.index === index);
          if (!token) return;
          const range = rangeForOffsets(this.contentDOM, token.start, token.end);
          if (!range) return;
          const rect = range.getBoundingClientRect();
          if (rect.width === 0 && rect.height === 0) return;
          const center = rect.left - rootRect.left + rect.width / 2;
          const top =
            mode === "sotto"
              ? rect.bottom - rootRect.top + 3
              : rect.top - rootRect.top - space + 3;
          chip.style.left = `${center}px`;
          chip.style.top = `${top}px`;
        });
      });
    }

    renderStrip(force = false) {
      const all = tokensByPid.get(this.pid) ?? [];
      // Con la CAA automatica spenta restano visibili solo le scelte manuali.
      // Se l'utente disattiva i simboli espliciti, anche quelle vengono nascoste
      // dal foglio ma restano salvate nel documento.
      const manual = (token: CaaToken) => token.locked || token.strategy === "manual";
      const explicit = (token: CaaToken) =>
        isExplicitContent(token.surface, token.symbol_label, token.concept_label, token.meaning);
      const source = autoEnabled ? all : all.filter(manual);
      const filtered = showExplicitSymbols ? source : source.filter((token) => !explicit(token));
      const visible = stripTokens(filtered);
      const hidden = mode === "testo" || visible.length === 0;
      // La chiave contiene solo ciò che cambia l'aspetto: niente SVG (grande
      // e immutabile) e niente ricostruzione quando il testo viene digitato.
      const key = hidden
        ? `${this.pid}|hidden|${mode}|${autoEnabled}|${showExplicitSymbols}|${symbolLayout}`
        : `${this.pid}|${mode}|${autoEnabled}|${showExplicitSymbols}|${symbolLayout}|${appliedKey.startsWith(`${this.pid}:`) ? appliedKey : ""}|${visible
            .map(
              (token) =>
                `${token.index}:${token.symbol_id ?? ""}:${token.composition.length}:${
                  token.locked ? 1 : 0
                }:${token.hidden ? 1 : 0}:${token.approximate ? 1 : 0}:${
                  token.alternatives.some((alternative) => alternative.neural) ? 1 : 0
                }`
            )
            .join(";")}`;
      if (!force && key === this.lastKey) {
        this.order();
        return;
      }
      this.lastKey = key;
      if (hidden) {
        this.strip.replaceChildren();
        this.strip.hidden = true;
      } else {
        this.strip.hidden = false;
        this.strip.dataset.layout = mode === "sotto" ? "sotto" : "sopra";
        renderChips(this.strip, this.pid, visible);
      }
      this.order();
      this.positionChips();
    }

    /// Aggiorna solo lo stato di selezione dei chip: evita di ricostruire gli
    /// SVG (e lo sfarfallio) a ogni clic.
    updateSelection() {
      const chips = this.strip.querySelectorAll<HTMLElement>(".caa-chip");
      chips.forEach((chip) => {
        const index = Number(chip.dataset.index);
        const active = Boolean(
          selectedUnit && selectedUnit.pid === this.pid && selectedUnit.index === index
        );
        chip.classList.toggle("caa-chip--selected", active);
        if (active) chip.setAttribute("aria-current", "true");
        else chip.removeAttribute("aria-current");
      });
    }

    update(node: PMNode): boolean {
      if (node.type !== this.node.type) return false;
      if (this.tag === "heading" && node.attrs.level !== this.node.attrs.level) return false;
      this.node = node;
      this.syncPid();
      this.applyAttrs();
      // Anche se il testo cambia, i token restano quelli dell'ultima analisi:
      // ridisegnare qui sarebbe lavoro inutile a ogni tasto. Il ridisegno
      // avviene quando arriva un risultato nuovo o cambia la selezione.
      this.renderStrip();
      this.positionChips();
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
      if (this.positionFrame !== null) cancelAnimationFrame(this.positionFrame);
      window.removeEventListener("resize", this.onResize);
      if (paragraphViews.get(this.pid) === this) paragraphViews.delete(this.pid);
    }
  }

  function textViewFor(tag: "paragraph" | "heading") {
    return (node: PMNode) => new TextView(node, tag);
  }

  function refreshStrips(force = false, only?: Set<string>) {
    for (const [pid, paragraphView] of paragraphViews) {
      if (only && !only.has(pid)) continue;
      paragraphView.renderStrip(force);
    }
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
        keydown: () => {
          lastKeyAt = performance.now();
          return false;
        },
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
      profile_id: profileId,
    };
  }

  /// Arricchimento automatico con ARASAAC. Parte solo dopo l'analisi locale,
  /// in background: il testo resta subito scrivibile. Le parole sono singole,
  /// minuscole e ripulite; nomi propri, frasi, numeri, email e URL restano fuori.
  async function ensureOnlineSymbols() {
    if (!onlineAutoInsert || !autoEnabled || mode === "testo" || !isTauri() || onlineBusy) {
      return;
    }
    const words = new Set<string>();
    const pidsByWord = new Map<string, Set<string>>();
    const normalize = (value: string) =>
      value
        .toLocaleLowerCase("it")
        .normalize("NFD")
        .replace(/[\u0300-\u036f]/g, "")
        .trim();
    for (const [pid, tokens] of tokensByPid) {
      for (const token of tokens) {
        if (
          token.is_punct ||
          token.is_function ||
          token.composition.length > 0 ||
          token.locked ||
          token.strategy === "manual"
        ) {
          continue;
        }
        const surface = token.surface.trim();
        // Priorità ARASAAC: anche un simbolo locale non ARASAAC viene
        // affiancato/cercato online; un simbolo con etichetta diversa dalla
        // parola (es. «amare» per «volare») è un candidato alla sostituzione.
        if (token.symbol_id) {
          const label = normalize(token.symbol_label ?? token.concept_label ?? "");
          const word = normalize(surface);
          const lemma = normalize(token.lemma ?? "");
          const labelMatches = label.length > 0 && (label === word || label === lemma);
          // Copie ARASAAC o personali già esatte: niente rete.
          if ((token.pack_id === "arasaac" || token.pack_id === "personal") && labelMatches) {
            continue;
          }
        }
        const lower = surface.toLocaleLowerCase("it");
        // Le parole con maiuscola iniziale possono essere nomi propri:
        // l'inserimento automatico non li invia alla rete.
        if (!surface || surface !== lower) continue;
        if (lower.length < 2 || lower.length > 40) continue;
        if (!/^[\p{L}][\p{L}'’\-]*$/u.test(lower)) continue;
        if (onlineAttempted.has(lower)) continue;
        words.add(lower);
        if (!pidsByWord.has(lower)) pidsByWord.set(lower, new Set());
        pidsByWord.get(lower)?.add(pid);
      }
    }
    const pending = [...words].slice(0, 6);
    if (pending.length === 0) return;
    for (const word of pending) onlineAttempted.add(word);
    onlineBusy = true;
    try {
      const exact = await api.ensureOnlineSymbols(pending);
      const remaining = pending.filter((word) => !exact.includes(word));
      // Se ARASAAC non ha un'etichetta esatta, si importa comunque il primo
      // pittogramma ARASAAC disponibile per la parola: la scrittura continua
      // con un simbolo, poi l'utente può scegliere un'alternativa migliore.
      const filled = remaining.length > 0 ? await api.onlineAutoFill(remaining) : [];
      const saved = [...exact, ...filled];
      if (saved.length > 0) {
        // I simboli sono ora nel database: la prossima analisi li trova per
        // etichetta esatta. Si invalidano solo i paragrafi coinvolti, non
        // l'intero documento, per non innescare rianalisi inutili.
        let touched = false;
        for (const word of saved) {
          for (const pid of pidsByWord.get(word) ?? []) {
            analyzedSignature.delete(pid);
            touched = true;
          }
        }
        if (touched) setTimeout(() => schedule(60), 0);
      }

      // Le parole ancora senza simbolo passano al ripiego automatico:
      // sinonimi italiani → Global Symbols → sinonimi inglesi. La pausa
      // lascia finire la digitazione e tiene il processo fuori dal percorso
      // critico; se l'utente continua a scrivere, i token vengono ricontrollati.
      const unresolved = pending.filter((word) => !saved.includes(word));
      if (unresolved.length > 0) {
        await new Promise((resolve) => window.setTimeout(resolve, 1200));
        const resolved = await api.resolveMissingSymbols(unresolved);
        let touched = false;
        for (const entry of resolved) {
          const alternative = entry.alternative;
          let mediaSvg = alternative.svg;
          let mediaImage = alternative.image;
          if (!mediaSvg && !mediaImage && alternative.symbol_id) {
            try {
              const hydrated = await api.hydrateSymbols([alternative.symbol_id]);
              const value = hydrated[alternative.symbol_id];
              if (value) {
                if (value.startsWith("data:")) mediaImage = value;
                else mediaSvg = value;
              }
            } catch {
              /* file non disponibile: resta il segnaposto */
            }
          }
          for (const pid of pidsByWord.get(entry.word) ?? []) {
            const tokens = tokensByPid.get(pid) ?? [];
            let pidTouched = false;
            const next = tokens.map((token) => {
              if (
                token.symbol_id ||
                token.locked ||
                normalize(token.surface) !== entry.word
              ) {
                return token;
              }
              pidTouched = true;
              return {
                ...token,
                symbol_id: alternative.symbol_id,
                pack_id: alternative.pack_id,
                svg: mediaSvg ?? null,
                image: mediaImage ?? null,
                symbol_label: alternative.label,
                concept_id: alternative.concept_id ?? token.concept_id,
                concept_label: alternative.label,
                meaning: alternative.meaning,
                strategy: "related",
                confidence: 0.35,
                approximate: true,
              };
            });
            if (!pidTouched) continue;
            tokensByPid.set(pid, next);
            analyzedSignature.delete(pid);
            paragraphViews.get(pid)?.renderStrip();
            const applied = next.find(
              (token) =>
                normalize(token.surface) === entry.word &&
                token.symbol_id === alternative.symbol_id
            );
            if (applied) {
              await api
                .setDocumentOverride(
                  documentKey,
                  `${pid}:${applied.index}`,
                  alternative.symbol_id,
                  alternative.concept_id
                )
                .catch(() => undefined);
            }
            touched = true;
          }
        }
        if (touched) {
          onAnalyzed(new Map(tokensByPid));
          window.setTimeout(() => schedule(60), 0);
        }
      }
    } catch {
      // La rete è facoltativa: un errore non interrompe la scrittura.
    } finally {
      onlineBusy = false;
    }
  }

  async function analyzeAll() {
    if (!view) return;
    if (analyzing) {
      queued = true;
      return;
    }
    const run = ++analyzeSeq;
    analyzing = true;
    onAnalysisState?.(true);
    const startedAt = performance.now();
    try {
      let paragraphs = collectParagraphs();
      // Il paragrafo in cui si sta scrivendo viene analizzato per primo: in un
      // documento lungo il pittogramma della parola corrente non deve aspettare
      // tutti i paragrafi precedenti.
      const activePid = activeParagraphPid();
      if (activePid) {
        paragraphs = [
          ...paragraphs.filter((paragraph) => paragraph.pid === activePid),
          ...paragraphs.filter((paragraph) => paragraph.pid !== activePid),
        ];
      }
      if (!autoEnabled) {
        // I token restano in memoria: le scelte manuali devono poter essere
        // mostrate e modificate anche con la CAA automatica spenta.
        refreshStrips(true);
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
          // Una risposta arrivata dopo che il testo è cambiato non deve
          // sovrascrivere la versione nuova.
          if (run !== analyzeSeq) return;
          const current = collectParagraphs().find((item) => item.pid === paragraph.pid);
          if (!current || signatureFor(current) !== signature) continue;
          analyzedSignature.set(paragraph.pid, signature);
          tokensByPid.set(paragraph.pid, result.tokens);
          paragraphViews.get(paragraph.pid)?.renderStrip();
        } catch (error) {
          if (run !== analyzeSeq) return;
          analyzedSignature.set(paragraph.pid, signature);
          const message = error instanceof Error ? error.message : String(error);
          if (!message.includes("Anteprima browser")) onError(message);
        }
      }
      if (run !== analyzeSeq) return;
      onAnalyzed(new Map(tokensByPid));
      refreshStrips();
      scheduleRefine();
      void ensureOnlineSymbols();
    } finally {
      analyzing = false;
      if (run === analyzeSeq) {
        analysisRuns += 1;
        lastAnalysisMs = performance.now() - startedAt;
        onAnalysisState?.(false);
        reportMetrics();
      }
      if (queued) {
        queued = false;
        schedule(80);
      }
    }
  }

  /// Paragrafo della selezione corrente, per dare priorità alla parola che si
  /// sta scrivendo.
  function activeParagraphPid(): string | null {
    const selection = view?.state.selection;
    const node = selection?.$from.parent;
    const pid = node?.attrs?.pid;
    return typeof pid === "string" && pid ? pid : null;
  }

  function schedule(delay = Math.min(analysisDebounceMs, 180)) {
    // Ogni nuovo testo invalida le risposte in volo: il risultato vecchio
    // verrà scartato anche se arriva dopo.
    analyzeSeq += 1;
    if (refineTimer) {
      clearTimeout(refineTimer);
      refineTimer = null;
    }
    void cancelCurrentRefine();
    onAnalysisState?.(true);
    if (debounce) clearTimeout(debounce);
    debounce = setTimeout(() => {
      debounce = null;
      void analyzeAll();
    }, Math.max(40, delay));
  }

  function cancelCurrentRefine() {
    const key = refineRequestId;
    refineRequestId = "";
    if (key && isTauri()) {
      return api.cancelRefine(key).catch(() => undefined);
    }
    return Promise.resolve();
  }

  /// Passata neurale facoltativa: parte dopo una breve pausa di scrittura, senza
  /// bloccare la digitazione. Ogni nuova analisi annulla la passata precedente; il
  /// risultato si applica solo se il testo non è cambiato.
  function scheduleRefine(delay = refineDebounceMs) {
    if (refineTimer) clearTimeout(refineTimer);
    refineTimer = setTimeout(() => {
      refineTimer = null;
      void refineAll();
    }, Math.max(120, delay));
  }

  async function refineAll() {
    if (!autoEnabled || !neural || !isTauri() || !view) return;
    const seq = ++refineSeq;
    onAnalysisState?.(true);
    const startedAt = performance.now();
    try {
      for (const paragraph of collectParagraphs()) {
        if (seq !== refineSeq) return;
        const text = paragraph.text.trim();
        // Basta un testo con almeno tre parole: aspettare il punto finale
        // significava non vedere mai la correzione contestuale mentre si scrive.
        if (!text || text.split(/\s+/).filter(Boolean).length < 3) continue;
        const signature = signatureFor(paragraph);
        if (refinedSignature.get(paragraph.pid) === signature) continue;
        const requestId = `${paragraph.pid}:${seq}`;
        refineRequestId = requestId;
        try {
          const result = await api.refineParagraph(paragraph.text, options(paragraph), requestId);
          if (seq !== refineSeq) return;
          const current = collectParagraphs().find((item) => item.pid === paragraph.pid);
          if (!current || signatureFor(current) !== signature) continue;
          refinedSignature.set(paragraph.pid, signature);
          tokensByPid.set(paragraph.pid, result.tokens);
          paragraphViews.get(paragraph.pid)?.renderStrip();
          onAnalyzed(new Map(tokensByPid));
        } catch {
          // Modello assente, richiesta annullata o testo superato: resta valida
          // la scelta lessicale, senza avvisi e senza toccare il documento.
          if (seq !== refineSeq) return;
          refinedSignature.set(paragraph.pid, signature);
        } finally {
          if (refineRequestId === requestId) refineRequestId = "";
        }
      }
    } finally {
      if (seq === refineSeq) {
        lastAnalysisMs = performance.now() - startedAt;
        onAnalysisState?.(false);
        reportMetrics();
      }
    }
  }

  /// Invio: nuova voce negli elenchi, nuovo paragrafo altrove. Non invia
  /// prompt né conferma suggerimenti: il cursore va semplicemente a capo.
  /// I mark (dimensione, colore, grassetto…) restano attivi nel nuovo
  /// blocco: senza questo ProseMirror li azzera all'inizio del paragrafo,
  /// perché non c'è testo precedente da cui ereditarli (effetto percepito
  /// come «la dimensione non resta quando vado a capo»).
  function splitBlockOrList(state: EditorState, dispatch?: (tr: Transaction) => void) {
    const marks = state.storedMarks ?? state.selection.$from.marks();
    const split =
      splitListItem(schema.nodes.list_item)(state, dispatch) || splitBlock(state, dispatch);
    if (split && dispatch && view && marks.length > 0) {
      const tr = view.state.tr.setStoredMarks(
        marks.map((mark) => mark.type.create(mark.attrs))
      );
      dispatch(tr);
    }
    return split;
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
        const updateStarted = performance.now();
        const next = view!.state.apply(transaction);
        view!.updateState(next);
        if (transaction.docChanged) {
          const withIds = ensurePids(view!.state);
          if (withIds !== view!.state) view!.updateState(withIds);
          onDocChanged?.();
          schedule();
        }
        reportFormatting();
        const elapsed = performance.now() - updateStarted;
        updateSamples.push(elapsed);
        if (updateSamples.length > 200) updateSamples.shift();
        if (lastKeyAt) {
          inputSamples.push(performance.now() - lastKeyAt);
          if (inputSamples.length > 200) inputSamples.shift();
          lastKeyAt = 0;
          // Il reporting avviene al massimo una volta ogni 40 transazioni per
          // non aggiungere lavoro al percorso di battitura.
          if (updateSamples.length % 40 === 0) reportMetrics();
        }
      },
      attributes: {
        class: "ProseMirror editor",
        role: "textbox",
        "aria-multiline": "true",
        "aria-label": "Documento CAA",
        spellcheck: "true",
      },
    });

    // Lacune di frame superiori a 120 ms indicano un blocco percepibile
    // dell'interfaccia. Sono misurate nel WebView, non dedotte.
    let frameHandle = 0;
    let lastFrame = performance.now();
    const watchFrames = () => {
      const now = performance.now();
      if (document.visibilityState === "visible" && now - lastFrame > 120) {
        longTasks += 1;
        longestTaskMs = Math.max(longestTaskMs, now - lastFrame);
      }
      lastFrame = now;
      frameHandle = requestAnimationFrame(watchFrames);
    };
    frameHandle = requestAnimationFrame(watchFrames);

    reportFormatting();
    schedule(30);
    return () => {
      cancelAnimationFrame(frameHandle);
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
    void symbolLayout;
    void showExplicitSymbols;
    if (!view) return;
    refreshStrips();
  });

  // Evidenzia il pittogramma selezionato aggiornando solo i due paragrafi
  // coinvolti (quello che perde e quello che riceve la selezione). Prima si
  // ricostruivano tutti i chip del documento a ogni clic.
  let previousSelectionKey = "";
  $effect(() => {
    const nextKey = selectedUnit ? `${selectedUnit.pid}:${selectedUnit.index}` : "";
    if (!view) return;
    if (nextKey === previousSelectionKey) return;
    const touched = new Set<string>();
    if (previousSelectionKey) touched.add(previousSelectionKey.split(":")[0]);
    if (selectedUnit) touched.add(selectedUnit.pid);
    previousSelectionKey = nextKey;
    if (touched.size === 0) {
      for (const view of paragraphViews.values()) view.updateSelection();
    } else {
      for (const pid of touched) paragraphViews.get(pid)?.updateSelection();
    }
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
    refineSeq += 1;
    void cancelCurrentRefine();
    schedule(0);
  }

  /// Applica subito un pittogramma scelto nel pannello, senza aspettare la
  /// rianalisi: il foglio si aggiorna nello stesso istante in cui l'utente
  /// sceglie il suggerimento. Il testo non viene toccato.
  /// Token corrente per indice: serve a verificare che la scelta del pannello
  /// non venga applicata a una parola diversa dopo una rianalisi.
  /// Riallinea il paragrafo con i token già noti al chiamante. Serve quando
  /// una scelta manuale arriva mentre l'editor non ha ancora la mappa interna
  /// (per esempio subito dopo l'apertura di un documento).
  export function setTokensForPid(pid: string, tokens: CaaToken[]): void {
    if (!tokens.length) return;
    tokensByPid.set(pid, tokens);
    analyzedSignature.delete(pid);
    refreshStrips(true, new Set([pid]));
    onAnalyzed(new Map(tokensByPid));
  }

  /// Sostituisce il testo dell'unità mantenendo il pittogramma corrente.
  /// Usato dal comando Ribbon «Cambia testo»: il simbolo resta bloccato,
  /// l'override di occorrenza viene salvato e la rianalisi è immediata.
  export async function changeTokenText(pid: string, index: number, value: string): Promise<boolean> {
    const text = value.trim();
    if (!view || !text) return false;
    const block = blockForPid(pid);
    const token = (tokensByPid.get(pid) ?? []).find((item) => item.index === index);
    if (!block || !token || !token.symbol_id) return false;
    const from = block.pos + 1 + token.start;
    const to = block.pos + 1 + token.end;
    if (from < 0 || to < from || to > view.state.doc.content.size) return false;
    const symbolId = token.symbol_id;
    const conceptId = token.concept_id;
    const hidden = token.hidden;
    const modifiers = token.modifiers ?? {
      plural: false,
      negation: false,
      past: false,
      future: false,
      manual: false,
    };
    view.dispatch(view.state.tr.insertText(text, from, to));
    // Mantiene il pittogramma e lo blocca: la rianalisi non deve sostituirlo.
    const key = `${pid}:${index}`;
    await api
      .setDocumentOverride(documentKey, key, symbolId, conceptId ?? null)
      .catch(() => undefined);
    await api
      .setUnitState(documentKey, key, true, hidden, modifiers)
      .catch(() => undefined);
    const tokens = tokensByPid.get(pid);
    if (tokens) {
      tokensByPid.set(
        pid,
        tokens.map((item) =>
          item.index === index
            ? { ...item, surface: text, caption: text, locked: true }
            : item
        )
      );
    }
    analyzedSignature.delete(pid);
    refreshStrips(true, new Set([pid]));
    onAnalyzed(new Map(tokensByPid));
    schedule(0);
    return true;
  }

  /// Inserisce le immagini scaricate on demand nei token che ne erano privi
  /// (tipicamente ARASAAC dopo la rimozione del catalogo locale). I valori
  /// `data:` sono PNG usati direttamente; gli altri restano SVG.
  export function hydrateSymbols(media: Record<string, string>): void {
    let changed = false;
    for (const [pid, tokens] of tokensByPid) {
      let touched = false;
      const next = tokens.map((token) => {
        const value = token.symbol_id ? media[token.symbol_id] : undefined;
        if (
          token.symbol_id &&
          !token.image &&
          value &&
          (token.pack_id === "arasaac" || !token.svg)
        ) {
          touched = true;
          return value.startsWith("data:")
            ? { ...token, image: value }
            : { ...token, svg: value };
        }
        return token;
      });
      if (touched) {
        tokensByPid.set(pid, next);
        analyzedSignature.delete(pid);
        changed = true;
      }
    }
    if (!changed) return;
    refreshStrips(true);
    onAnalyzed(new Map(tokensByPid));
    if (selectedUnit) {
      const token = tokensByPid
        .get(selectedUnit.pid)
        ?.find((item) => item.index === selectedUnit.index);
      if (token) onSelectToken(selectedUnit.pid, token);
    }
  }

  export function tokenAt(pid: string, index: number): CaaToken | null {
    return tokensByPid.get(pid)?.find((token) => token.index === index) ?? null;
  }

  export function applyTokenSymbol(
    pid: string,
    index: number,
    alternative: {
      symbol_id: string;
      svg: string | null;
      image?: string | null;
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
      image: alternative.image ?? null,
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
    // Ridisegno forzato: il chip appena scelto deve comparire subito, anche se
    // un'analisi in corso ha lasciato la chiave della striscia invariata.
    refreshStrips(true, new Set([pid]));
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
        image?: string | null;
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
        image: entry.alternative.image ?? null,
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
      refreshStrips(true, new Set(entries.map((entry) => entry.pid)));
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

  /// Annulla l'ultima transazione dell'editor (usato dal pannello Assistente).
  export function undoEdit(): boolean {
    if (!view) return false;
    const done = undo(view.state, view.dispatch);
    if (done) view.focus();
    return done;
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

  /// Cancella il carattere prima del cursore: serve alla tastiera visuale,
  /// dove il tasto «Cancella» non può generare un backspace nativo.
  export function backspace() {
    if (!view) return;
    const { from, to } = view.state.selection;
    if (from !== to) {
      view.dispatch(view.state.tr.deleteSelection());
    } else if (from > 1) {
      view.dispatch(view.state.tr.delete(from - 1, from));
    }
    view.focus();
  }

  export function insertTextAtSelection(text: string) {
    if (!view || !text) return;
    view.dispatch(view.state.tr.insertText(text));
    view.focus();
  }

  /// Nuovo paragrafo dalla tastiera visuale (il tasto Invio fisico funziona
  /// già, ma un pulsante touch non può generare una transazione ProseMirror).
  export function insertParagraph() {
    if (!view) return;
    splitBlockOrList(view.state, view.dispatch);
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

  /// Applica colore/dimensione. Chiave assente = conserva il valore
  /// esistente; `null` esplicito = azzera. Con cursore senza selezione la
  /// formattazione resta come «stile di digitazione» (stored marks),
  /// come in LibreOffice: prima la funzione usciva subito e "talvolta
  /// non funzionava proprio".
  export function applyTextStyle(patch: { color?: string | null; font_size?: string | null }) {
    if (!view) return;
    const state = view.state;
    const { from, to, empty } = state.selection;
    const existing = (state.storedMarks ?? state.selection.$from.marks()).find(
      (mark) => mark.type.name === "text_style"
    );
    const mark = schema.marks.text_style.create({
      color: "color" in patch ? patch.color ?? null : (existing?.attrs.color ?? null),
      font_size:
        "font_size" in patch ? patch.font_size ?? null : (existing?.attrs.font_size ?? null),
    });
    const tr = state.tr;
    if (empty) {
      tr.addStoredMark(mark);
    } else {
      tr.removeMark(from, to, schema.marks.text_style).addMark(from, to, mark);
    }
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
  data-captions={mode === "sotto" || ["immagine_testo", "separato", "divisore"].includes(borderStyle) ? "true" : "false"}
  data-border={borderStyle}
></div>
