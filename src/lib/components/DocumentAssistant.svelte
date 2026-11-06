<script lang="ts">
  import { api } from "../api";
  import type { DocumentEditPlan, DocumentOperation } from "../types";

  interface Props {
    onClose: () => void;
    onApply: (operations: DocumentOperation[]) => { applied: number; warnings: string[] };
    onUndo: () => boolean;
    neuralReady: boolean;
  }

  let { onClose, onApply, onUndo, neuralReady }: Props = $props();

  let prompt = $state("");
  let scope = $state("document");
  let useNeural = $state(true);
  let busy = $state(false);
  let error = $state("");
  let plan = $state<DocumentEditPlan | null>(null);
  let history = $state<
    {
      prompt: string;
      descriptions: string[];
      source: string;
      applied: number;
      warnings: string[];
      undone: boolean;
    }[]
  >([]);
  let canUndo = $state(false);

  const scopes = [
    { id: "selection", label: "Selezione" },
    { id: "paragraph", label: "Paragrafo" },
    { id: "page", label: "Pagina" },
    { id: "document", label: "Documento" },
    { id: "table", label: "Tabella" },
    { id: "cell", label: "Cella" },
    { id: "project", label: "Intero progetto" },
  ];

  async function interpret() {
    if (!prompt.trim() || busy) return;
    busy = true;
    error = "";
    try {
      plan = await api.interpretDocumentPrompt(prompt.trim(), useNeural);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function apply() {
    if (!plan || plan.operations.length === 0) return;
    const outcome = onApply(plan.operations);
    history = [
      {
        prompt: prompt.trim(),
        descriptions: plan.descriptions,
        source: plan.source,
        applied: outcome.applied,
        warnings: outcome.warnings,
        undone: false,
      },
      ...history,
    ].slice(0, 20);
    canUndo = outcome.applied > 0;
    plan = null;
    prompt = "";
  }

  function undo() {
    if (!canUndo) return;
    if (onUndo()) {
      canUndo = false;
      history = history.map((item, index) =>
        index === 0 ? { ...item, undone: true } : item
      );
    }
  }
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onClose()} />

<div class="dialog-backdrop">
  <div class="report-dialog assistant-dialog" role="dialog" aria-modal="true" aria-label="Modifica con IA">
    <header class="report-dialog__head">
      <h2>Modifica con IA</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
    </header>

    <p class="hint">
      Scrivi cosa vuoi cambiare. Il modello propone solo operazioni supportate e validate;
      non esegue codice e non modifica il documento finché non premi Applica.
    </p>

    <label class="assistant-field">
      Richiesta
      <textarea
        bind:value={prompt}
        rows="3"
        placeholder="Es. Ingrandisci i pittogrammi del 20% e metti il testo in maiuscolo"
        onkeydown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) void interpret();
        }}
      ></textarea>
    </label>

    <div class="assistant-row">
      <label>
        Ambito
        <select bind:value={scope}>
          {#each scopes as item (item.id)}
            <option value={item.id}>{item.label}</option>
          {/each}
        </select>
      </label>
      <label class="assistant-check">
        <input type="checkbox" bind:checked={useNeural} disabled={!neuralReady} />
        Usa il modello locale se le regole non bastano
      </label>
    </div>
    {#if !neuralReady}
      <p class="hint">Nessun modello neurale attivo: restano disponibili le regole deterministiche.</p>
    {:else}
      <p class="hint">
        Ambito dichiarato: <strong>{scopes.find((item) => item.id === scope)?.label}</strong>.
        Le operazioni di questa versione agiscono sul documento o sulla tabella corrente.
      </p>
    {/if}

    <div class="assistant-actions">
      <button class="btn btn--solid" onclick={() => void interpret()} disabled={busy || !prompt.trim()}>
        {busy ? "Interpretazione…" : "Interpreta"}
      </button>
      <span class="hint">Ctrl/Cmd+Invio per interpretare</span>
    </div>

    {#if error}
      <p class="import-error">{error}</p>
    {/if}

    {#if plan}
      <section class="assistant-preview">
        <h3>Anteprima delle modifiche</h3>
        {#if plan.descriptions.length === 0}
          <p class="hint">Nessuna operazione proposta.</p>
        {:else}
          <ol>
            {#each plan.descriptions as line (line)}
              <li>{line}</li>
            {/each}
          </ol>
        {/if}
        {#each plan.warnings as warning (warning)}
          <p class="import-error">{warning}</p>
        {/each}
        <p class="hint">Fonte: {plan.source === "neurale" ? "modello locale" : "regole deterministiche"}.</p>
        <div class="assistant-actions">
          <button
            class="btn btn--solid"
            onclick={apply}
            disabled={plan.operations.length === 0}
          >Applica</button>
          <button class="btn btn--ghost" onclick={() => (plan = null)}>Annulla</button>
        </div>
      </section>
    {/if}

    {#if canUndo}
      <button class="btn btn--ghost" onclick={undo}>Annulla l'ultima modifica applicata</button>
    {/if}

    {#if history.length > 0}
      <section class="assistant-history">
        <h3>Cronologia</h3>
        {#each history as item, index (index)}
          <div class="assistant-history__item">
            <div>
              <strong>{item.prompt}</strong>
              <span class="hint"> · {item.applied} operazioni · {item.source}</span>
              {#if item.undone}<span class="hint"> · annullata</span>{/if}
            </div>
            <div class="hint">{item.descriptions.join(" · ")}</div>
            {#each item.warnings as warning (warning)}
              <div class="import-error">{warning}</div>
            {/each}
          </div>
        {/each}
      </section>
    {/if}
  </div>
</div>
