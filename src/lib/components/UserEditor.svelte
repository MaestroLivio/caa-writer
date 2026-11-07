<script lang="ts">
  import { onMount } from "svelte";
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import CaaEditor from "../editor/CaaEditor.svelte";
  import { api, isTauri } from "../api";
  import { recordRecent } from "../recent";
  import type { Alternative, CaaToken, DocumentSettings, Profile } from "../types";
  import { defaultSettings, noModifiers } from "../types";

  interface Props {
    profile: Profile;
    onBack: () => void;
    onSpeak: (text: string) => void;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
  }

  let { profile, onBack, onSpeak, onToast }: Props = $props();

  const level = Math.max(1, Math.min(3, Number(profile.editor_level) || 1));
  const levelLabel =
    level === 1 ? "Livello 1 · Solo simboli" : level === 2 ? "Livello 2 · Scrivo con aiuto" : "Livello 3 · Scrivo";
  const canFormat = level >= 3;
  const canType = level >= 2;
  const canSpeak = profile.features.includes("speech");

  let editor = $state<CaaEditor | undefined>(undefined);
  let title = $state("I miei pensieri");
  let vocabulary = $state<Alternative[]>([]);
  let vocabularyBusy = $state(true);
  let vocabularyError = $state("");
  let tokensByParagraph = $state<Map<string, CaaToken[]>>(new Map());
  let analysisActive = $state(false);
  let saving = $state(false);
  let docPath = $state<string | null>(null);

  type Unit = { key: string; label: string; svg: string | null; symbolId: string | null };
  let message = $state<Unit[]>([]);
  let unitSeq = 0;

  const documentKey = `user-${profile.id}-${Date.now().toString(36)}`;

  const documentSettings: DocumentSettings = {
    ...defaultSettings,
    mode: "sopra",
    intensity: level === 1 ? "minima" : "media",
    auto_caa: true,
    neural_context: false,
    show_captions: level >= 2,
    font_size: level === 1 ? 20 : 18,
    symbol_size: level === 1 ? 84 : level === 2 ? 68 : 56,
    symbolize_articles: false,
    symbolize_prepositions: false,
    symbolize_conjunctions: false,
    theme: (profile.age_theme as DocumentSettings["theme"]) || "chiaro",
    touch_mode: true,
  };

  onMount(() => {
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

  function addUnit(alternative: Alternative) {
    unitSeq += 1;
    message = [
      ...message,
      {
        key: `u${unitSeq}`,
        label: alternative.label,
        svg: alternative.svg,
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
        if (token.symbol_id && token.svg && !seen.has(token.symbol_id)) {
          seen.add(token.symbol_id);
          assets.push({ symbol_id: token.symbol_id, svg: token.svg, png_base64: null });
        }
      }
    }
    for (const unit of message) {
      if (unit.symbolId && unit.svg && !seen.has(unit.symbolId)) {
        seen.add(unit.symbolId);
        assets.push({ symbol_id: unit.symbolId, svg: unit.svg, png_base64: null });
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

  const letterRows = [
    "qwertyuiop".split(""),
    "asdfghjkl".split(""),
    "zxcvbnm".split(""),
  ];
  const accentRow = ["à", "è", "é", "ì", "ò", "ù", "'", "?", "!", ".", ","];
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
              <button class="user-unit" onclick={() => removeUnit(unit.key)} title="Rimuovi">
                {#if unit.svg}<span class="user-unit__media">{@html unit.svg}</span>{/if}
                <span>{unit.label} ×</span>
              </button>
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
              {#if alternative.svg}
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
    <main class="user-editor__writing">
      <section class="user-paper" aria-label="Area di scrittura">
        <CaaEditor
          bind:this={editor}
          mode="sopra"
          intensity="media"
          documentKey={documentKey}
          commercialOnly={false}
          borderStyle={level >= 2 ? "immagine_testo" : "immagine"}
          packId={null}
          selectedUnit={null}
          autoEnabled={true}
          neural={false}
          symbolizeArticles={false}
          symbolizePrepositions={false}
          symbolizeConjunctions={false}
          profileId={profile.id}
          analysisDebounceMs={280}
          refineDebounceMs={900}
          onSelectToken={() => {}}
          onTokenMenu={() => {}}
          onError={(error) => onToast(error, "error")}
          onAnalyzed={(map) => (tokensByParagraph = map)}
          onAnalysisState={(active) => (analysisActive = active)}
        />
      </section>

      <aside class="user-assist" aria-label="Strumenti di scrittura">
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
                {#if alternative.svg}
                  <span class="user-word__media">{@html alternative.svg}</span>
                {/if}
                <span>{alternative.label}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="user-assist__group">
          <span class="hint">Tastiera</span>
          <div class="user-keyboard">
            {#each letterRows as row}
              <div class="user-keyboard__row">
                {#each row as letter}
                  <button class="user-key" onclick={() => editor?.insertTextAtSelection(letter)}>
                    {letter}
                  </button>
                {/each}
              </div>
            {/each}
            <div class="user-keyboard__row">
              {#each accentRow as key}
                <button class="user-key" onclick={() => editor?.insertTextAtSelection(key)}>
                  {key}
                </button>
              {/each}
            </div>
            <div class="user-keyboard__row">
              <button class="user-key user-key--wide" onclick={() => editor?.insertTextAtSelection(" ")}>
                Spazio
              </button>
              <button class="user-key user-key--wide" onclick={() => editor?.insertParagraph()}>
                Invio
              </button>
              <button class="user-key user-key--wide" onclick={() => editor?.backspace()}>
                Cancella
              </button>
            </div>
          </div>
        </div>

        <p class="hint">
          Questa è la vista Utilizzatore: i comandi del Creator non sono disponibili. Il documento
          salvato si apre con «Apri documento».
        </p>
      </aside>
    </main>
  {/if}
</div>

<style>
  .user-editor {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px clamp(12px, 3vw, 28px) 24px;
    background: var(--bg, #f6f7fb);
    color: var(--text, #16161a);
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
    display: grid;
    grid-template-columns: minmax(260px, 0.9fr) minmax(320px, 1.6fr);
    gap: 14px;
    min-height: 0;
  }

  .user-message {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 14px;
    background: #fff;
  }

  .user-message__units {
    flex: 1 1 auto;
    min-height: 200px;
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

  .user-unit__media :global(svg) {
    width: 52px;
    height: 52px;
  }

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

  .user-symbol__media :global(svg) {
    width: 64px;
    height: 64px;
  }

  .user-symbol__label {
    font-size: 0.85rem;
    font-weight: 600;
    text-align: center;
  }

  .user-editor__writing {
    display: grid;
    grid-template-columns: minmax(0, 1.5fr) minmax(280px, 0.9fr);
    gap: 14px;
    align-items: start;
    min-height: 0;
  }

  .user-paper {
    background: #fff;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 12px;
    padding: 18px;
    min-height: 55vh;
  }

  .user-assist {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    border: 1px solid var(--border, #d3d6e0);
    border-radius: 12px;
    background: #fff;
    max-height: 76vh;
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

  .user-word__media :global(svg) {
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
    .user-editor__level-one,
    .user-editor__writing {
      grid-template-columns: minmax(0, 1fr);
    }
    .user-assist {
      max-height: none;
    }
  }
</style>
