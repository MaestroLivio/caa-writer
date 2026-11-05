<script lang="ts">
  import { onMount } from "svelte";
  import { EditorState, Plugin } from "prosemirror-state";
  import type { Transaction } from "prosemirror-state";
  import { EditorView } from "prosemirror-view";
  import type { NodeView, ViewMutationRecord } from "prosemirror-view";
  import type { Node as PMNode } from "prosemirror-model";
  import { history, undo, redo } from "prosemirror-history";
  import { keymap } from "prosemirror-keymap";
  import { baseKeymap, setBlockType, splitBlock, toggleMark } from "prosemirror-commands";
  import {
    liftListItem,
    sinkListItem,
    splitListItem,
    wrapInList,
  } from "prosemirror-schema-list";
  import { schema } from "./schema";
  import { tokenForSelection } from "./selection";
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

  function signatureFor(text: string): string {
    return `${text}\u0001${mode}\u0001${intensity}\u0001${commercialOnly}\u0001${packId ?? ""}\u0001${documentKey}\u0001${symbolizeArticles}\u0001${symbolizePrepositions}\u0001${symbolizeConjunctions}`;
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
      const align = this.node.attrs.align as string | null;
      this.contentDOM.style.textAlign = align ?? "";
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

  function selectionFromView(current: EditorView) {
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

  function collectParagraphs(): { pid: string; text: string; style: string }[] {
    if (!view) return [];
    const out: { pid: string; text: string; style: string }[] = [];
    view.state.doc.descendants((node) => {
      if (node.type.name === "paragraph" || node.type.name === "heading") {
        out.push({
          pid: String(node.attrs.pid ?? "detached"),
          text: node.textContent,
          style: node.type.name === "heading" ? "heading" : "paragraph",
        });
      }
    });
    return out;
  }

  function options(pid: string) {
    return {
      intensity,
      mode,
      document_key: documentKey,
      commercial_only: commercialOnly,
      pack_id: packId,
      paragraph_id: pid,
      symbolize_articles: symbolizeArticles,
      symbolize_prepositions: symbolizePrepositions,
      symbolize_conjunctions: symbolizeConjunctions,
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
        const signature = signatureFor(paragraph.text);
        if (analyzedSignature.get(paragraph.pid) === signature) continue;
        budget -= 1;
        try {
          const result = isTauri()
            ? await api.symbolizeParagraph(paragraph.text, options(paragraph.pid))
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
      const signature = signatureFor(paragraph.text);
      if (refinedSignature.get(paragraph.pid) === signature) continue;
      try {
        const result = await api.refineParagraph(paragraph.text, options(paragraph.pid));
        if (seq !== refineSeq) return;
        const current = collectParagraphs().find((item) => item.pid === paragraph.pid);
        if (!current || signatureFor(current.text) !== signature) continue;
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

  onMount(() => {
    let state = EditorState.create({
      doc: initialDoc(),
      plugins: [
        history(),
        selectionSyncPlugin,
        keymap({
          "Mod-z": undo,
          "Mod-y": redo,
          "Shift-Mod-z": redo,
          Enter: splitBlockOrList,
          Tab: sinkListItem(schema.nodes.list_item),
          "Shift-Tab": liftListItem(schema.nodes.list_item),
        }),
        keymap(baseKeymap),
      ],
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
          schedule();
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
      EditorState.create({ doc, plugins: [history(), selectionSyncPlugin] })
    );
    paragraphViews.clear();
    view.updateState(state);
    analyzedSignature.clear();
    refinedSignature.clear();
    tokensByPid.clear();
    void plain;
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
    view.updateState(EditorState.create({ doc, plugins: [history()] }));
    analyzedSignature.clear();
    refinedSignature.clear();
    tokensByPid.clear();
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
    refreshStrips();
    onAnalyzed(new Map(tokensByPid));
    onSelectToken(pid, updated);
    return true;
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
