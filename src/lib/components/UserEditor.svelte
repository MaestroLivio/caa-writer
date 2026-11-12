<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import CaaEditor from "../editor/CaaEditor.svelte";
  import FloatingKeyboard from "./FloatingKeyboard.svelte";
  import { api, isTauri } from "../api";
  import { recordRecent } from "../recent";
  import { canChooseAlternatives, symbolChoices } from "../user-symbols";
  import type { Alternative, CaaToken, DocumentSettings, Profile } from "../types";
  import { defaultSettings, noModifiers } from "../types";

  interface Props {
    profile: Profile;
    /// Impostazioni CAA del Creator: la modalità Utente usa le stesse regole
    /// di simbolizzazione del testo libero, non una configurazione parallela.
    settings?: DocumentSettings;
    onBack: () => void;
    onSpeak: (text: string) => void;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
  }

  let { profile, settings = defaultSettings, onBack, onSpeak, onToast }: Props = $props();

  const level = Math.max(1, Math.min(3, Number(profile.editor_level) || 1));
  const levelLabel =
    level === 1 ? "Livello 1 · Solo simboli" : level === 2 ? "Livello 2 · Scrivo con aiuto" : "Livello 3 · Scrivo";
  const canFormat = level >= 3;
  const canType = level >= 2;
  const canSpeak = profile.features.includes("speech");
  /// Le alternative per singolo pittogramma sono una ricerca guidata: si
  /// attivano solo se il profilo ha il permesso «Ricerca pittogrammi».
  const canAlternatives = canChooseAlternatives(profile);
  const effectiveMode = $derived(settings.auto_caa ? settings.mode : "testo");

  let editor = $state<CaaEditor | undefined>(undefined);
  let title = $state("I miei pensieri");
  let vocabulary = $state<Alternative[]>([]);
  let vocabularyBusy = $state(true);
  let vocabularyError = $state("");
  let tokensByParagraph = $state<Map<string, CaaToken[]>>(new Map());
  let analysisActive = $state(false);
  let saving = $state(false);
  let docPath = $state<string | null>(null);

  type Unit = {
    key: string;
    label: string;
    svg: string | null;
    image: string | null;
    symbolId: string | null;
  };
  let message = $state<Unit[]>([]);
  let unitSeq = 0;

  /// Parola selezionata nel testo: il pannello Simboli in basso mostra le sue
  /// alternative con lo stesso ordine del word processor.
  let selectedToken = $state<{ pid: string; token: CaaToken } | null>(null);
  let symbolQuery = $state("");
  let symbolResults = $state<Alternative[]>([]);
  let symbolBusy = $state(false);
  let symbolError = $state("");

  /// Riquadro di destra: minimizzabile e ridimensionabile. La scelta è
  /// ricordata per profilo, così ogni utilizzatore ritrova la sua larghezza.
  const assistKey = `caa.user.assist.${profile.id}`;
  let assistOpen = $state(true);
  let assistWidth = $state(320);
  let resizingAssist = false;
  let imageInput: HTMLInputElement | undefined = $state();

  /// Pannello Simboli in basso: ridimensionabile e minimizzabile come il
  /// riquadro di destra. Anche questa preferenza è legata al profilo.
  const symbolsKey = `caa.user.symbols.${profile.id}`;
  let symbolsOpen = $state(true);
  let symbolsHeight = $state(300);
  let resizingSymbols = false;

  function clampSymbols(value: number): number {
    const max = Math.max(180, Math.round(window.innerHeight * 0.7));
    return Math.min(max, Math.max(140, Math.round(value)));
  }

  function persistSymbols() {
    try {
      localStorage.setItem(
        symbolsKey,
        JSON.stringify({ open: symbolsOpen, height: symbolsHeight })
      );
    } catch {
      /* la preferenza vale per questa sessione */
    }
  }

  function toggleSymbols() {
    symbolsOpen = !symbolsOpen;
    persistSymbols();
  }

  function startSymbolsResize(event: PointerEvent) {
    event.preventDefault();
    resizingSymbols = true;
    const startY = event.clientY;
    const startHeight = symbolsHeight;
    const move = (moveEvent: PointerEvent) => {
      // Il bordo di ridimensionamento è in alto: trascinando verso l'alto il
      // pannello cresce, verso il basso si riduce.
      symbolsHeight = clampSymbols(startHeight + (startY - moveEvent.clientY));
    };
    const up = () => {
      resizingSymbols = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      persistSymbols();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function clampAssist(value: number): number {
    return Math.min(520, Math.max(220, Math.round(value)));
  }

  function persistAssist() {
    try {
      localStorage.setItem(
        assistKey,
        JSON.stringify({ open: assistOpen, width: assistWidth })
      );
    } catch {
      /* la preferenza vale per questa sessione */
    }
  }

  function toggleAssist() {
    assistOpen = !assistOpen;
    persistAssist();
  }

  function startAssistResize(event: PointerEvent) {
    event.preventDefault();
    resizingAssist = true;
    const startX = event.clientX;
    const startWidth = assistWidth;
    const move = (moveEvent: PointerEvent) => {
      assistWidth = clampAssist(startWidth + (startX - moveEvent.clientX));
    };
    const up = () => {
      resizingAssist = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      persistAssist();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  function chooseImage() {
    imageInput?.click();
  }

  async function onImageChosen(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    try {
      const data = await new Promise<string>((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => resolve(String(reader.result ?? ""));
        reader.onerror = () => reject(reader.error ?? new Error("file non leggibile"));
        reader.readAsDataURL(file);
      });
      const label = file.name.replace(/\.[^.]+$/, "");
      editor?.insertImage(data, label);
      onToast("Immagine inserita nel punto del cursore.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  /// Sintesi durante la digitazione: stesse regole dell'editor universale.
  function onTypingKey(event: KeyboardEvent) {
    if (!canType || !canSpeak) return;
    const target = event.target as HTMLElement | null;
    if (!target?.closest(".user-paper .ProseMirror")) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (!isTauri()) return;
    const sentenceTrigger = ["Enter", ".", "!", "?", ":", ";"].includes(event.key);
    const wordTrigger = ["Enter", " "].includes(event.key);
    if (settings.speak_sentences && sentenceTrigger) {
      if (event.key === "Enter") {
        const sentence = editor?.currentSentenceText() ?? "";
        if (sentence.trim()) onSpeak(sentence);
      } else {
        window.setTimeout(() => {
          const sentence = editor?.sentenceBeforeCursor() ?? "";
          if (sentence.trim()) onSpeak(sentence);
        }, 0);
      }
      return;
    }
    if (settings.speak_words && wordTrigger) {
      if (event.key === "Enter") {
        const text = editor?.currentBlockText() ?? "";
        const match = text.match(/[\p{L}\p{N}][\p{L}\p{N}'’\-]*\s*$/u);
        if (match) onSpeak(match[0].trim());
      } else {
        const word = editor?.wordBeforeCursor();
        if (word) onSpeak(word);
      }
    }
  }

  const liveToken = $derived.by((): CaaToken | null => {
    if (!selectedToken) return null;
    const tokens = tokensByParagraph.get(selectedToken.pid);
    return tokens?.find((token) => token.index === selectedToken!.token.index) ?? selectedToken.token;
  });

  const tokenAlternatives = $derived.by((): Alternative[] => symbolChoices(liveToken));

  function selectToken(pid: string, token: CaaToken) {
    if (!canAlternatives) return;
    selectedToken = { pid, token };
    symbolQuery = token.surface;
    symbolResults = token.alternatives ?? [];
    symbolError = "";
  }

  function closeTokenSelection() {
    selectedToken = null;
    symbolResults = [];
    symbolQuery = "";
    symbolError = "";
  }

  async function searchPanelSymbols() {
    const query = symbolQuery.trim();
    if (!query || !isTauri()) {
      symbolError = query
        ? "La ricerca dei pittogrammi richiede l'app desktop."
        : "Scrivi una parola da cercare.";
      return;
    }
    symbolBusy = true;
    symbolError = "";
    try {
      const page = await api.searchSymbols(
        query,
        24,
        0,
        settings.pack_id,
        settings.commercial_only
      );
      symbolResults = page.items;
      if (page.items.length === 0) symbolError = `Nessun pittogramma per «${query}».`;
    } catch (error) {
      symbolResults = [];
      symbolError = error instanceof Error ? error.message : String(error);
    } finally {
      symbolBusy = false;
    }
  }

  /// Applica un simbolo alla parola selezionata; senza selezione inserisce la
  /// parola nel punto del cursore e lascia che il motore la simbolizzi.
  function applyPanelSymbol(alternative: Alternative) {
    const target = selectedToken;
    const token = liveToken;
    if (target && token && editor && canAlternatives) {
      editor.applyTokenSymbol(target.pid, token.index, {
        symbol_id: alternative.symbol_id,
        svg: alternative.svg,
        image: alternative.image ?? null,
        label: alternative.label,
        meaning: alternative.meaning,
        concept_id: alternative.concept_id,
        approximate: alternative.approximate,
      });
    } else {
      insertWord(alternative);
    }
    symbolResults = [];
  }

  function toggleSelectedLock() {
    const target = selectedToken;
    const token = liveToken;
    if (!target || !token) return;
    editor?.applyTokenState(target.pid, token.index, { locked: !token.locked });
  }

  function toggleSelectedHidden() {
    const target = selectedToken;
    const token = liveToken;
    if (!target || !token) return;
    editor?.applyTokenState(target.pid, token.index, { hidden: !token.hidden });
  }

  type AltTarget =
    | { kind: "unit"; key: string }
    | { kind: "token"; pid: string; index: number; surface: string };
  let altTarget = $state<AltTarget | null>(null);
  let altResults = $state<Alternative[]>([]);
  let altBusy = $state(false);
  let altError = $state("");

  /// Apre il pannello delle alternative per una parola. Non cambia il testo:
  /// la scelta sostituisce solo il pittogramma.
  async function openAlternatives(target: AltTarget, query: string) {
    if (!canAlternatives) return;
    altTarget = target;
    altError = "";
    altResults = [];
    if (!query.trim() || !isTauri()) {
      altError = isTauri()
        ? "Questa parola non ha un pittogramma da cambiare."
        : "Le alternative richiedono l'app desktop.";
      return;
    }
    altBusy = true;
    try {
      const page = await api.searchSymbols(query.trim(), 24, 0, null, false);
      altResults = page.items;
      if (page.items.length === 0) altError = "Nessuna alternativa trovata.";
    } catch (error) {
      altError = error instanceof Error ? error.message : String(error);
    } finally {
      altBusy = false;
    }
  }

  function closeAlternatives() {
    altTarget = null;
    altResults = [];
    altError = "";
  }

  function chooseAlternative(alternative: Alternative) {
    const target = altTarget;
    if (!target) return;
    if (target.kind === "unit") {
      message = message.map((unit) =>
        unit.key === target.key
          ? {
              ...unit,
              label: unit.label,
              svg: alternative.svg,
              image: alternative.image ?? null,
              symbolId: alternative.symbol_id,
            }
          : unit
      );
    } else {
      editor?.applyTokenSymbol(target.pid, target.index, {
        symbol_id: alternative.symbol_id,
        svg: alternative.svg,
        image: alternative.image ?? null,
        label: alternative.label,
        meaning: alternative.meaning,
        concept_id: alternative.concept_id,
        approximate: alternative.approximate,
      });
    }
    closeAlternatives();
  }

  const documentKey = `user-${profile.id}-${Date.now().toString(36)}`;

  const documentSettings: DocumentSettings = {
    ...defaultSettings,
    ...settings,
    mode: settings.mode,
    intensity: settings.intensity,
    neural_context: settings.neural_context && profile.features.includes("ai"),
    show_captions: level >= 2,
    font_size: level === 1 ? 20 : settings.font_size,
    symbol_size: level === 1 ? 84 : settings.symbol_size,
    theme: (profile.age_theme as DocumentSettings["theme"]) || settings.theme,
    touch_mode: true,
  };

  onMount(() => {
    try {
      const raw = localStorage.getItem(assistKey);
      if (raw) {
        const parsed = JSON.parse(raw) as { open?: boolean; width?: number };
        assistOpen = parsed.open !== false;
        assistWidth = clampAssist(Number(parsed.width) || 320);
      }
    } catch {
      /* preferenza non disponibile */
    }
    try {
      const raw = localStorage.getItem(symbolsKey);
      if (raw) {
        const parsed = JSON.parse(raw) as { open?: boolean; height?: number };
        symbolsOpen = parsed.open !== false;
        symbolsHeight = clampSymbols(Number(parsed.height) || 300);
      }
    } catch {
      /* preferenza non disponibile */
    }
    window.addEventListener("keydown", onTypingKey, true);
    if (!isTauri()) {
      vocabularyBusy = false;
      vocabularyError = "Vocabolario disponibile solo nell'app desktop.";
      return;
    }
    void (async () => {
      try {
        const loaded = await api.commonSymbols(48);
        vocabulary = Array.isArray(loaded) ? loaded : [];
        if (vocabulary.length === 0) {
          vocabularyError = "Nessun vocabolo di base con pittogramma nel repertorio.";
        }
      } catch (error) {
        vocabularyError = error instanceof Error ? error.message : String(error);
      } finally {
        vocabularyBusy = false;
      }
    })();
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onTypingKey, true);
  });

  function addUnit(alternative: Alternative) {
    unitSeq += 1;
    message = [
      ...message,
      {
        key: `u${unitSeq}`,
        label: alternative.label,
        svg: alternative.svg,
        image: alternative.image ?? null,
        symbolId: alternative.symbol_id,
      },
    ];
    if (level === 1 && canSpeak) onSpeak(alternative.label);
  }

  function removeUnit(key: string) {
    message = message.filter((unit) => unit.key !== key);
  }

  function speakMessage() {
    const text = message.map((unit) => unit.label).join(" ").trim();
    if (text) onSpeak(text);
  }

  function insertWord(alternative: Alternative) {
    editor?.insertTextAtSelection(`${alternative.label} `);
  }

  function speakEditor() {
    const text = editor?.getPlainText().trim() ?? "";
    if (text) onSpeak(text);
  }

  function clearDocument() {
    if (canType) {
      editor?.insertPlainText("");
    } else {
      message = [];
    }
  }

  function buildLevelOneContent() {
    return {
      type: "doc",
      // Le unità del livello 1 conservano il pittogramma scelto: alla
      // riapertura la frase ricompare com'era, non come testo senza simboli.
      user_units: message.map((unit) => ({
        label: unit.label,
        symbol_id: unit.symbolId,
        svg: unit.svg,
        image: unit.image,
      })),
      content: message.map((unit, index) => ({
        type: "paragraph",
        attrs: { pid: `user-${Date.now().toString(36)}-${index}` },
        content: unit.label ? [{ type: "text", text: unit.label }] : [],
      })),
    };
  }

  function collectAssets() {
    const assets: { symbol_id: string; svg: string | null; png_base64: string | null }[] = [];
    const seen = new Set<string>();
    for (const tokens of tokensByParagraph.values()) {
      for (const token of tokens) {
        if (token.symbol_id && (token.svg || token.image) && !seen.has(token.symbol_id)) {
          seen.add(token.symbol_id);
          const dataUrl = token.image ?? token.svg;
          const png = dataUrl?.startsWith("data:") ? dataUrl.split(",", 2)[1] ?? null : null;
          assets.push({
            symbol_id: token.symbol_id,
            svg: png ? null : token.svg,
            png_base64: png,
          });
        }
      }
    }
    for (const unit of message) {
      if (unit.symbolId && (unit.svg || unit.image) && !seen.has(unit.symbolId)) {
        seen.add(unit.symbolId);
        const png = unit.image?.startsWith("data:")
          ? (unit.image.split(",", 2)[1] ?? null)
          : null;
        assets.push({
          symbol_id: unit.symbolId,
          svg: png ? null : unit.svg,
          png_base64: png,
        });
      }
    }
    return assets;
  }

  function collectPreferences() {
    return [...tokensByParagraph.entries()].flatMap(([pid, tokens]) =>
      tokens
        .filter((token) => token.locked || token.hidden || (token.modifiers?.manual ?? false))
        .map((token) => ({
          unit_key: `${pid}:${token.index}`,
          symbol_id: token.symbol_id,
          concept_id: token.concept_id,
          locked: token.locked,
          hidden: token.hidden,
          modifiers: token.modifiers ?? { ...noModifiers },
        }))
    );
  }

  /// Apre un documento scritto con questo editor. I materiali della libreria
  /// e i fogli liberi del Creator non appartengono a questa sezione: si
  /// aprono dall'Area Creator, dove hanno i comandi completi.
  async function openDocument() {
    if (!isTauri()) {
      onToast("L'apertura dei documenti richiede l'app desktop.", "warn");
      return;
    }
    const picked = await openDialog({
      title: "Apri un mio documento",
      multiple: false,
      filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
    });
    if (!picked) return;
    try {
      const loaded = await api.loadDocument(String(picked));
      const layout = loaded.layout as { document_type?: string } | null;
      if (loaded.universal || layout?.document_type === "free_board") {
        onToast(
          "Questo documento non è stato scritto con l'editor Utilizzatore: aprilo dall'Area Creator.",
          "warn"
        );
        return;
      }
      title = loaded.title || "I miei pensieri";
      docPath = loaded.path || String(picked);
      if (canType) {
        editor?.setDocJson(loaded.content, loaded.plain_text);
      } else {
        type SavedUnit = {
          label?: string;
          symbol_id?: string | null;
          svg?: string | null;
          image?: string | null;
        };
        const content = loaded.content as { user_units?: SavedUnit[] } | null;
        const units = content?.user_units;
        const saved: SavedUnit[] = Array.isArray(units)
          ? units
          : loaded.plain_text
              .split(/\n+/)
              .map((line) => line.trim())
              .filter(Boolean)
              .map((label) => ({ label }));
        const assets = new Map(loaded.assets.map((asset) => [asset.symbol_id, asset]));
        message = saved.map((unit) => {
          const label = unit.label ?? "";
          const asset = unit.symbol_id ? assets.get(unit.symbol_id) : undefined;
          const image =
            unit.image ??
            (asset?.png_base64 ? `data:image/png;base64,${asset.png_base64}` : null);
          return {
            key: `u${++unitSeq}`,
            label,
            svg: unit.svg ?? asset?.svg ?? null,
            image,
            symbolId: unit.symbol_id ?? asset?.symbol_id ?? null,
          };
        });
      }
      recordRecent({
        path: docPath,
        title,
        excerpt: loaded.plain_text.slice(0, 160),
      });
      loaded.warnings.forEach((warning) => onToast(warning, "warn"));
      onToast("Documento aperto.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function save() {
    if (!isTauri()) {
      onToast("Il salvataggio richiede l'app desktop.", "warn");
      return;
    }
    const content = canType ? editor?.getDocJson() : buildLevelOneContent();
    const plain = canType
      ? editor?.getPlainText() ?? ""
      : message.map((unit) => unit.label).join("\n");
    if (!content || (!plain.trim() && message.length === 0)) {
      onToast("Scrivi o scegli almeno un simbolo prima di salvare.", "warn");
      return;
    }
    let path = docPath;
    if (!path) {
      const picked = await saveDialog({
        title: "Salva il mio documento",
        defaultPath: `${title || "i-miei-pensieri"}.caadoc`,
        filters: [{ name: "Documento CAA Writer", extensions: ["caadoc"] }],
      });
      if (!picked) return;
      path = picked as string;
    }
    saving = true;
    try {
      const result = await api.saveDocument(path, {
        title: title || "I miei pensieri",
        content,
        plain_text: plain,
        settings: documentSettings,
        assets: collectAssets(),
        preferences: collectPreferences(),
        layout: null,
      });
      docPath = path;
      recordRecent({ path, title: title || "I miei pensieri", excerpt: plain.slice(0, 160) });
      if (result.warnings.length) result.warnings.forEach((warning) => onToast(warning, "warn"));
      onToast("Documento salvato.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      saving = false;
    }
  }
</script>

<div class="user-editor" data-level={level} data-theme={profile.age_theme}>
  <header class="user-editor__bar">
    <button class="btn" onclick={onBack}>← Indietro</button>
    <label class="user-editor__title">
      <span class="hint">Titolo</span>
      <input bind:value={title} aria-label="Titolo del documento" />
    </label>
    <span class="user-editor__level">{levelLabel}</span>
    <span class="grow"></span>
    {#if analysisActive}
      <span class="user-editor__status" role="status">Analisi in corso…</span>
    {/if}
    <button class="btn" onclick={clearDocument}>Svuota</button>
    {#if canSpeak}
      <button class="btn" onclick={() => (canType ? speakEditor() : speakMessage())}>Ascolta</button>
    {/if}
    <button class="btn" onclick={() => void openDocument()}>Apri</button>
    <button class="btn btn--solid" disabled={saving} onclick={() => void save()}>
      {saving ? "Salvo…" : "Salva"}
    </button>
  </header>

  {#if level === 1}
    <main class="user-editor__level-one">
      <section class="user-message" aria-label="La mia frase">
        <div class="user-message__units">
          {#if message.length === 0}
            <span class="hint">Tocca un pittogramma per costruire la frase.</span>
          {:else}
            {#each message as unit (unit.key)}
              <div class="user-unit-wrap">
                <button class="user-unit" onclick={() => removeUnit(unit.key)} title="Rimuovi">
                  {#if unit.image}
                    <span class="user-unit__media"><img src={unit.image} alt="" /></span>
                  {:else if unit.svg}<span class="user-unit__media">{@html unit.svg}</span>{/if}
                  <span>{unit.label} ×</span>
                </button>
                {#if canAlternatives}
                  <button
                    class="user-unit__alt"
                    onclick={() =>
                      void openAlternatives({ kind: "unit", key: unit.key }, unit.label)}
                    title="Cambia pittogramma"
                    aria-label={`Cambia il pittogramma di ${unit.label}`}
                  >↻</button>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
        <div class="user-message__actions">
          <button class="btn" onclick={() => (message = [])} disabled={message.length === 0}>
            Svuota
          </button>
          {#if canSpeak}
            <button class="btn btn--solid" onclick={speakMessage} disabled={message.length === 0}>
              Parla la frase
            </button>
          {/if}
        </div>
      </section>

      <section class="user-vocabulary" aria-label="Pittogrammi">
        {#if vocabularyBusy}
          <p class="hint">Carico il vocabolario…</p>
        {:else if vocabularyError}
          <p class="warn">{vocabularyError}</p>
        {:else}
          {#each vocabulary as alternative (alternative.symbol_id)}
            <button class="user-symbol" onclick={() => addUnit(alternative)}>
              {#if alternative.image}
                <span class="user-symbol__media"><img src={alternative.image} alt="" /></span>
              {:else if alternative.svg}
                <span class="user-symbol__media">{@html alternative.svg}</span>
              {:else}
                <span class="caa-chip__empty">?</span>
              {/if}
              <span class="user-symbol__label">{alternative.label}</span>
            </button>
          {/each}
        {/if}
      </section>
    </main>
  {:else}
    <main
      class="user-editor__writing"
      class:user-editor__writing--collapsed={!assistOpen}
      style={`--assist-width:${assistWidth}px`}
    >
      <section class="user-paper" aria-label="Area di scrittura">
        <CaaEditor
          bind:this={editor}
          mode={effectiveMode}
          intensity={settings.intensity}
          documentKey={documentKey}
          commercialOnly={settings.commercial_only}
          borderStyle={settings.border_style}
          packId={settings.pack_id}
          selectedUnit={selectedToken
            ? { pid: selectedToken.pid, index: selectedToken.token.index }
            : null}
          autoEnabled={settings.auto_caa}
          neural={settings.neural_context && profile.features.includes("ai")}
          onlineAutoInsert={settings.online_auto_insert && settings.online_search !== "off"}
          symbolizeArticles={settings.symbolize_articles}
          symbolizePrepositions={settings.symbolize_prepositions}
          symbolizeConjunctions={settings.symbolize_conjunctions}
          showExplicitSymbols={settings.show_explicit_symbols}
          symbolLayout={settings.symbol_layout}
          profileId={profile.id}
          analysisDebounceMs={settings.analysis_debounce_ms}
          refineDebounceMs={settings.refine_debounce_ms}
          onSelectToken={(pid, token) => selectToken(pid, token)}
          onTokenMenu={(pid, token) => selectToken(pid, token)}
          onError={(error) => onToast(error, "error")}
          onAnalyzed={(map) => (tokensByParagraph = map)}
          onAnalysisState={(active) => (analysisActive = active)}
        />
      </section>

      {#if !assistOpen}
        <button class="user-assist__restore" onclick={toggleAssist} title="Mostra i suggerimenti">
          <span aria-hidden="true">‹</span> Suggerimenti
        </button>
      {:else}
      <aside class="user-assist" aria-label="Strumenti di scrittura">
        <div class="user-assist__resize" role="separator" aria-orientation="vertical" aria-label="Ridimensiona i suggerimenti" onpointerdown={startAssistResize}></div>
        <header class="user-assist__head">
          <strong>Suggerimenti</strong>
          <button class="btn btn--ghost" onclick={toggleAssist} aria-label="Minimizza i suggerimenti" title="Minimizza">×</button>
        </header>
        <div class="user-assist__body">
        {#if canFormat}
          <div class="user-assist__group">
            <span class="hint">Formato</span>
            <div class="user-assist__row">
              <button class="btn" onclick={() => editor?.toggleBold()}><strong>G</strong></button>
              <button class="btn" onclick={() => editor?.toggleItalic()}><em>C</em></button>
              <button class="btn" onclick={() => editor?.toggleUnderline()}><u>S</u></button>
              <button class="btn" onclick={() => editor?.undoEdit()}>Annulla</button>
            </div>
          </div>
        {/if}

        <div class="user-assist__group">
          <span class="hint">Parole frequenti</span>
          <div class="user-assist__words">
            {#each vocabulary.slice(0, 16) as alternative (alternative.symbol_id)}
              <button class="user-word" onclick={() => insertWord(alternative)}>
                {#if alternative.image}
                  <span class="user-word__media"><img src={alternative.image} alt="" /></span>
                {:else if alternative.svg}
                  <span class="user-word__media">{@html alternative.svg}</span>
                {/if}
                <span>{alternative.label}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="user-assist__group">
          <span class="hint">Immagine</span>
          <button class="btn" onclick={chooseImage}>Inserisci immagine</button>
          <input
            class="user-assist__file"
            type="file"
            accept="image/*"
            bind:this={imageInput}
            onchange={onImageChosen}
            aria-label="Scegli un'immagine da inserire"
          />
          <p class="hint">L'immagine viene inserita nel punto del cursore e salvata nel documento.</p>
        </div>

        <div class="user-assist__group">
          <span class="hint">Tastiera</span>
          <p class="hint">
            La tastiera virtuale è ridotta a un pulsante in basso a destra: aprile quando serve,
            trascinala dove non copre la frase e ridimensionala dal bordo.
          </p>
        </div>

        <p class="hint">
          I documenti scritti qui si salvano sul computer e si riaprono con «Apri» nella barra in
          alto. Tabelle e materiali del Creator restano nell'Area Creator.
        </p>
        </div>
      </aside>
      {/if}
    </main>

    {#if symbolsOpen}
      <section
        class="user-symbols"
        style={`height:${symbolsHeight}px`}
        aria-label="Pannello Simboli"
      >
        <div
          class="user-symbols__resize"
          role="separator"
          aria-orientation="horizontal"
          aria-label="Ridimensiona il pannello dei simboli"
          onpointerdown={startSymbolsResize}
        ></div>
        <header class="user-symbols__head">
          <div>
            <h2>Simboli</h2>
            <p class="hint">
              Seleziona una parola nel documento per cambiarne il pittogramma; senza selezione il
              simbolo scelto viene inserito nel punto del cursore.
            </p>
          </div>
          <div class="user-symbols__head-actions">
            {#if selectedToken}
              <button class="btn" onclick={closeTokenSelection}>Chiudi selezione</button>
            {/if}
            <button
              class="btn btn--ghost"
              onclick={toggleSymbols}
              aria-label="Minimizza il pannello dei simboli"
              title="Minimizza"
            >×</button>
          </div>
        </header>

        <div class="user-symbols__body">
      {#if liveToken}
        <div class="user-symbols__selected">
          <div>
            <strong>«{liveToken.surface}»</strong>
            <span class="hint">
              {liveToken.symbol_label ?? liveToken.concept_label ?? "nessun pittogramma"}
              {#if liveToken.lemma} · {liveToken.lemma}{/if}
            </span>
          </div>
          <div class="user-symbols__actions">
            <button
              class="btn"
              class:btn--solid={liveToken.locked}
              onclick={toggleSelectedLock}
            >{liveToken.locked ? "Sblocca" : "Blocca"}</button>
            <button
              class="btn"
              class:btn--solid={liveToken.hidden}
              onclick={toggleSelectedHidden}
            >{liveToken.hidden ? "Mostra" : "Nascondi"}</button>
          </div>
          {#if tokenAlternatives.length}
            <div class="user-symbols__grid" aria-label="Alternative della parola">
              {#each tokenAlternatives as alternative (alternative.symbol_id)}
                <button
                  class="user-symbols__item"
                  class:active={liveToken.symbol_id === alternative.symbol_id}
                  onclick={() => applyPanelSymbol(alternative)}
                  title={alternative.license || alternative.label}
                >
                  {#if alternative.image}
                    <span class="user-symbols__media"><img src={alternative.image} alt="" /></span>
                  {:else if alternative.svg}
                    <span class="user-symbols__media">{@html alternative.svg}</span>
                  {/if}
                  <span class="user-symbols__label">{alternative.label}</span>
                </button>
              {/each}
            </div>
          {:else}
            <p class="hint">Nessuna alternativa automatica: usa la ricerca qui sotto.</p>
          {/if}
        </div>
      {/if}

      <div class="user-symbols__search">
        <label class="user-symbols__query">
          <span class="hint">Cerca un pittogramma</span>
          <input
            bind:value={symbolQuery}
            placeholder="Es. gatto, mangiare, scuola…"
            onkeydown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void searchPanelSymbols();
              }
            }}
          />
        </label>
        <button class="btn btn--solid" disabled={symbolBusy} onclick={() => void searchPanelSymbols()}>
          {symbolBusy ? "Cerco…" : "Cerca"}
        </button>
      </div>
      {#if symbolError}<p class="warn">{symbolError}</p>{/if}

      {#if symbolResults.length}
        <div class="user-symbols__grid user-symbols__grid--search" aria-label="Risultati della ricerca">
          {#each symbolResults as alternative (alternative.symbol_id)}
            <button class="user-symbols__item" onclick={() => applyPanelSymbol(alternative)}>
              {#if alternative.image}
                <span class="user-symbols__media"><img src={alternative.image} alt="" /></span>
              {:else if alternative.svg}
                <span class="user-symbols__media">{@html alternative.svg}</span>
              {/if}
              <span class="user-symbols__label">{alternative.label}</span>
            </button>
          {/each}
        </div>
      {/if}

      <div class="user-symbols__vocabulary" aria-label="Parole frequenti">
        {#each vocabulary.slice(0, 40) as alternative (alternative.symbol_id)}
          <button
            class="user-symbols__item"
            onclick={() => applyPanelSymbol(alternative)}
            title={selectedToken ? `Applica a «${liveToken?.surface ?? ""}»` : `Inserisci ${alternative.label}`}
          >
            {#if alternative.image}
              <span class="user-symbols__media"><img src={alternative.image} alt="" /></span>
            {:else if alternative.svg}
              <span class="user-symbols__media">{@html alternative.svg}</span>
            {/if}
            <span class="user-symbols__label">{alternative.label}</span>
          </button>
        {/each}
      </div>
        </div>
      </section>
    {:else}
      <button class="user-symbols__restore" onclick={toggleSymbols} title="Mostra il pannello dei simboli">
        <span aria-hidden="true">⌃</span> Simboli
      </button>
    {/if}
  {/if}

  {#if canType}
    <FloatingKeyboard
      profileId={profile.id}
      onInsert={(text) => editor?.insertTextAtSelection(text)}
      onSpace={() => editor?.insertTextAtSelection(" ")}
      onEnter={() => editor?.insertParagraph()}
      onBackspace={() => editor?.backspace()}
    />
  {/if}

  {#if altTarget}
    <section class="user-alt" role="dialog" aria-label="Alternative del pittogramma">
      <header class="user-alt__bar">
        <strong>Cambia pittogramma</strong>
        <button class="btn" onclick={closeAlternatives} aria-label="Chiudi le alternative">×</button>
      </header>
      {#if altBusy}
        <p class="hint">Cerco le alternative…</p>
      {:else if altError}
        <p class="warn">{altError}</p>
      {:else}
        <div class="user-alt__grid">
          {#each altResults as alternative (alternative.symbol_id)}
            <button class="user-alt__item" onclick={() => chooseAlternative(alternative)}>
              {#if alternative.image}
                <span class="user-alt__media"><img src={alternative.image} alt="" /></span>
              {:else if alternative.svg}
                <span class="user-alt__media">{@html alternative.svg}</span>
              {/if}
              <span class="user-alt__label">{alternative.label}</span>
              <span class="user-alt__license">{alternative.license}</span>
            </button>
          {/each}
        </div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .user-editor {
    height: 100vh;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px clamp(12px, 3vw, 28px) 14px;
    background: var(--bg, #f6f7fb);
    color: var(--text, #16161a);
    overflow: hidden;
  }

  .user-editor__bar {
    display: flex;
    align-items: flex-end;
    flex-wrap: wrap;
    gap: 8px;
  }

  .user-editor__title {
    display: flex;
    flex-direction: column;
    min-width: 220px;
  }

  .user-editor__title input {
    font-size: 1.05rem;
    font-weight: 600;
  }

  .user-editor__level {
    padding: 5px 10px;
    border-radius: 999px;
    background: #e8eefc;
    color: #12234a;
    font-weight: 600;
    font-size: 0.8rem;
  }

  .user-editor__status {
    color: var(--muted, #667085);
    font-size: 0.8rem;
  }

  .grow {
    flex: 1 1 auto;
  }

  .user-editor__level-one {
    flex: 1 1 auto;
    display: grid;
    grid-template-columns: minmax(260px, 0.9fr) minmax(320px, 1.6fr);
    gap: 14px;
    min-height: 0;
  }

  .user-editor__writing {
    flex: 1 1 auto;
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--assist-width, 320px);
    gap: 12px;
    align-items: stretch;
    min-height: 0;
  }

  .user-editor__writing--collapsed {
    grid-template-columns: minmax(0, 1fr) 44px;
  }

  .user-assist {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    overflow: hidden;
    padding: 12px;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 14px;
    background: #fff;
  }

  .user-assist__body {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: auto;
    padding-right: 2px;
  }

  .user-assist__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .user-assist__head strong { font-size: 0.95rem; }

  .user-assist__resize {
    position: absolute;
    top: 0;
    left: 0;
    width: 8px;
    height: 100%;
    cursor: col-resize;
    touch-action: none;
    z-index: 2;
  }

  .user-assist__resize:hover { background: rgba(36, 99, 235, 0.12); }

  .user-assist__restore {
    align-self: start;
    min-height: 120px;
    writing-mode: vertical-rl;
    text-orientation: mixed;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 12px;
    background: #fff;
    color: inherit;
    font: inherit;
    cursor: pointer;
    padding: 8px 4px;
  }

  .user-assist__file { display: none; }

  .user-message {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
    padding: 12px;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 14px;
    background: #fff;
  }

  .user-message__units {
    flex: 1 1 auto;
    min-height: 200px;
    overflow: auto;
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 8px;
    padding: 8px;
    border: 2px dashed #c8cddb;
    border-radius: 10px;
  }

  .user-unit {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    border: 1px solid #d3d6e0;
    border-radius: 10px;
    background: #f8fafc;
    padding: 6px;
    cursor: pointer;
    color: inherit;
    font: inherit;
  }

  .user-unit__media :global(svg),
  .user-unit__media :global(img) {
    width: 52px;
    height: 52px;
  }

  .user-unit-wrap {
    position: relative;
    display: inline-flex;
  }

  .user-unit__alt {
    position: absolute;
    top: -8px;
    right: -8px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: 1px solid #d3d6e0;
    background: #fff;
    font-size: 0.95rem;
    line-height: 1;
    cursor: pointer;
    touch-action: manipulation;
  }

  .user-unit__alt:hover { background: #dbe7ff; }

  .user-alt {
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%);
    z-index: 130;
    width: min(720px, calc(100vw - 32px));
    max-height: 52vh;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: #fff;
    border: 1px solid #cfd8e3;
    border-radius: 14px;
    box-shadow: 0 18px 44px rgba(15, 23, 42, 0.25);
  }

  .user-alt__bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .user-alt__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(92px, 1fr));
    gap: 8px;
    overflow: auto;
    padding: 2px;
  }

  .user-alt__item {
    display: grid;
    gap: 2px;
    justify-items: center;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    background: #fff;
    padding: 6px;
    cursor: pointer;
    font: inherit;
    color: inherit;
    min-height: 96px;
  }

  .user-alt__item:hover { background: #f1f5f9; }

  .user-alt__media :global(svg),
  .user-alt__media :global(img) {
    width: 48px;
    height: 48px;
    object-fit: contain;
  }

  .user-alt__label {
    font-size: 0.78rem;
    font-weight: 600;
    text-align: center;
  }

  .user-alt__license { font-size: 0.6rem; color: #64748b; text-align: center; }

  .user-message__actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .user-vocabulary {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(104px, 1fr));
    gap: 10px;
    align-content: start;
    min-height: 0;
    overflow: auto;
    padding: 4px;
  }

  .user-symbol {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    min-height: 104px;
    padding: 8px;
    border: 2px solid #cdd3df;
    border-radius: 12px;
    background: #fff;
    color: #16161a;
    cursor: pointer;
    font: inherit;
  }

  .user-symbol:hover,
  .user-symbol:focus-visible {
    border-color: #2463eb;
    box-shadow: 0 4px 12px rgba(36, 99, 235, 0.16);
  }

  .user-symbol__media :global(svg),
  .user-symbol__media :global(img) {
    width: 64px;
    height: 64px;
  }

  .user-symbol__label {
    font-size: 0.85rem;
    font-weight: 600;
    text-align: center;
  }

  .user-paper {
    background: #fff;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 12px;
    padding: 18px;
    min-height: 0;
    overflow: auto;
  }

  .user-assist__group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .user-assist__row {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .user-assist__words {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .user-word {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border: 1px solid #d3d6e0;
    border-radius: 8px;
    background: #f8fafc;
    cursor: pointer;
    color: inherit;
    font: inherit;
    font-size: 0.8rem;
  }

  .user-word__media :global(svg),
  .user-word__media :global(img) {
    width: 26px;
    height: 26px;
  }

  .user-keyboard {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .user-keyboard__row {
    display: flex;
    gap: 4px;
    justify-content: center;
  }

  .user-key {
    min-width: 34px;
    min-height: 36px;
    border: 1px solid #cdd3df;
    border-radius: 8px;
    background: #f8fafc;
    color: #16161a;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .user-key--wide {
    min-width: 76px;
  }

  @media (max-width: 980px) {
    .user-editor {
      height: auto;
      min-height: 100vh;
      overflow: visible;
    }
    .user-editor__level-one,
    .user-editor__writing {
      grid-template-columns: minmax(0, 1fr);
    }
    .user-assist {
      max-height: none;
    }
    .user-symbols {
      height: auto !important;
      max-height: 60vh;
    }
  }

  .user-symbols {
    position: relative;
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 140px;
    padding: 10px 12px 12px;
    background: #fff;
    border: 1px solid #dbe3ee;
    border-radius: 14px;
    box-shadow: 0 6px 18px rgba(15, 23, 42, 0.06);
    overflow: hidden;
  }

  .user-symbols__resize {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 9px;
    cursor: row-resize;
    touch-action: none;
    z-index: 2;
  }

  .user-symbols__resize:hover { background: rgba(36, 99, 235, 0.12); }

  .user-symbols__body {
    flex: 1 1 auto;
    min-height: 0;
    display: grid;
    gap: 10px;
    align-content: start;
    overflow: auto;
  }

  .user-symbols__restore {
    flex: 0 0 auto;
    min-height: 42px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    border: 1px dashed #cbd5e1;
    border-radius: 12px;
    background: #fff;
    color: inherit;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }

  .user-symbols__restore:hover { background: #f1f5f9; }

  .user-symbols__head-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .user-symbols__head {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    align-items: flex-start;
    gap: 8px;
  }

  .user-symbols__head h2 {
    margin: 0;
    font-size: 1rem;
  }

  .user-symbols__head p {
    margin: 3px 0 0;
    max-width: 760px;
  }

  .user-symbols__selected {
    display: grid;
    gap: 8px;
    padding: 10px;
    border-radius: 10px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
  }

  .user-symbols__selected strong { font-size: 1rem; }

  .user-symbols__actions { display: flex; flex-wrap: wrap; gap: 6px; }

  .user-symbols__search {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px;
  }

  .user-symbols__query {
    display: grid;
    gap: 3px;
    flex: 1 1 240px;
  }

  .user-symbols__query input {
    min-height: 40px;
    border: 1px solid #cbd5e1;
    border-radius: 9px;
    padding: 7px 10px;
    font: inherit;
  }

  .user-symbols__grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(88px, 1fr));
    gap: 6px;
    max-height: 220px;
    overflow: auto;
    padding: 2px;
  }

  .user-symbols__grid--search { max-height: 180px; }

  .user-symbols__vocabulary {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding: 2px 2px 4px;
  }

  .user-symbols__vocabulary .user-symbols__item { flex: 0 0 88px; }

  .user-symbols__item {
    display: grid;
    gap: 2px;
    justify-items: center;
    min-height: 86px;
    padding: 6px;
    border: 1px solid #e2e8f0;
    border-radius: 10px;
    background: #fff;
    color: inherit;
    font: inherit;
    cursor: pointer;
    touch-action: manipulation;
  }

  .user-symbols__item:hover { background: #f1f5f9; }
  .user-symbols__item.active { border-color: #1f5bd8; background: #dbe7ff; }

  .user-symbols__media :global(svg),
  .user-symbols__media :global(img) {
    width: 44px;
    height: 44px;
    object-fit: contain;
  }

  .user-symbols__label {
    font-size: 0.72rem;
    font-weight: 600;
    text-align: center;
    line-height: 1.15;
  }

  @media (max-width: 720px) {
    .user-symbols__grid { grid-template-columns: repeat(auto-fill, minmax(72px, 1fr)); }
  }
</style>
