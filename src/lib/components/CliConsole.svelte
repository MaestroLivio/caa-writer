<script lang="ts">
  import { api } from "../api";
  import { classify } from "../decision/client";
  import { isNeuralMethod } from "../decision/ranking";
  import type { CliRunResult } from "../types";

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  const methods = [
    "info",
    "methods",
    "symbolize",
    "analyze.text",
    "search",
    "affinities",
    "templates.list",
    "templates.plan",
    "layout.build",
    "import.scan",
    "collections.list",
    "online.providers",
    "online.search",
    "prompt.interpret",
    "voice.status",
    "ai.status",
  ];

  const examples: { label: string; method: string; params: string }[] = [
    {
      label: "Simbolizza una frase",
      method: "symbolize",
      params: '{\n  "text": "La sintesi vocale legge il testo.",\n  "options": {"paragraph_id": "p1", "mode": "sopra", "intensity": "media"}\n}',
    },
    {
      label: "Cerca nella libreria",
      method: "search",
      params: '{"query": "cane", "limit": 10}',
    },
    {
      label: "Elenca i modelli",
      method: "templates.list",
      params: '{"category": "Tabelle comunicative"}',
    },
    {
      label: "Interpreta un prompt",
      method: "prompt.interpret",
      params: '{"prompt": "pittogrammi più grandi e testo maiuscolo", "use_neural": false}',
    },
  ];

  const methodLabels: Record<string, string> = {
    info: "mostrare le informazioni dell'applicazione",
    methods: "elencare i metodi disponibili",
    symbolize: "simbolizzare una frase con i pittogrammi",
    "analyze.text": "analizzare il testo",
    search: "cercare un pittogramma nella libreria",
    affinities: "trovare parole affini",
    "templates.list": "elencare i modelli didattici",
    "templates.plan": "pianificare un modello didattico",
    "layout.build": "costruire un layout",
    "import.scan": "scansionare immagini da importare",
    "collections.list": "elencare le collezioni",
    "online.providers": "elencare i provider online",
    "online.search": "cercare pittogrammi online",
    "prompt.interpret": "interpretare una richiesta di modifica",
    "voice.status": "mostrare lo stato della voce",
    "ai.status": "mostrare lo stato del modello IA",
  };

  let method = $state("info");
  let params = $state("{}");
  let busy = $state(false);
  let commandPrompt = $state("");
  let commandBusy = $state(false);
  let commandSuggestion = $state<{ id: string; label: string } | null>(null);
  let result = $state<CliRunResult | null>(null);
  let error = $state("");

  async function run() {
    if (busy) return;
    let parsed: unknown;
    try {
      parsed = params.trim() ? JSON.parse(params) : {};
    } catch (err) {
      error = `JSON non valido: ${err instanceof Error ? err.message : String(err)}`;
      return;
    }
    busy = true;
    error = "";
    result = null;
    try {
      result = await api.runCli(method.trim(), parsed);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function suggestCommand() {
    if (!commandPrompt.trim() || commandBusy) return;
    commandBusy = true;
    try {
      const response = await classify({
        task: "command_routing",
        input: { text: commandPrompt.trim() },
        candidates: methods.map((entry) => ({
          id: entry,
          label: methodLabels[entry] ?? entry,
          hypothesis: `La richiesta corrisponde al comando ${methodLabels[entry] ?? entry}.`,
        })),
        options: { mode: "suggest", multi_label: false },
      });
      if (
        response &&
        response.status === "completed" &&
        isNeuralMethod(response.metadata.method) &&
        response.selected_candidate_id
      ) {
        const top = response.ranked_candidates[0];
        commandSuggestion = {
          id: response.selected_candidate_id,
          label: top?.label ?? response.selected_candidate_id,
        };
      } else {
        commandSuggestion = null;
      }
    } finally {
      commandBusy = false;
    }
  }

  function useSuggestion() {
    if (!commandSuggestion) return;
    method = commandSuggestion.id;
    const example = examples.find((entry) => entry.method === commandSuggestion!.id);
    params = example?.params ?? "{}";
    commandSuggestion = null;
  }

  function useExample(example: (typeof examples)[number]) {
    method = example.method;
    params = example.params;
    result = null;
    error = "";
  }

  const output = $derived(
    result ? JSON.stringify(result.json ?? result.stdout ?? "", null, 2) : ""
  );
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") onClose();
    if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) void run();
  }}
/>

<div class="dialog-backdrop">
  <div class="report-dialog cli-dialog" role="dialog" aria-modal="true" aria-label="Console CLI">
    <header class="report-dialog__head">
      <h2>Console CLI</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
    </header>

    <p class="hint">
      Esegue <code>caa-cli</code> con gli stessi dati e le stesse risorse dell'app.
      I comandi che modificano dati (import, collezioni, online.save) hanno effetto immediato.
      La documentazione completa è in <code>docs/AGENT.md</code>.
    </p>

    <div class="cli-examples">
      {#each examples as example (example.label)}
        <button class="btn btn--ghost" onclick={() => useExample(example)}>{example.label}</button>
      {/each}
    </div>

    <div class="cli-route">
      <label>
        Descrivi cosa vuoi fare
        <input
          bind:value={commandPrompt}
          placeholder="Es. cerca il pittogramma del cane"
          onkeydown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void suggestCommand();
            }
          }}
        />
      </label>
      <button class="btn" onclick={() => void suggestCommand()} disabled={commandBusy || !commandPrompt.trim()}>
        {commandBusy ? "Analisi…" : "Suggerisci metodo"}
      </button>
      {#if commandSuggestion}
        <span class="hint">
          Metodo suggerito: <strong>{commandSuggestion.id}</strong> — {commandSuggestion.label}
          <button class="btn btn--ghost" onclick={useSuggestion}>Usa</button>
        </span>
      {/if}
    </div>

    <div class="cli-form">
      <label>
        Metodo
        <input list="cli-methods" bind:value={method} placeholder="es. info" />
        <datalist id="cli-methods">
          {#each methods as name (name)}
            <option value={name}></option>
          {/each}
        </datalist>
      </label>
      <label class="cli-params">
        Parametri JSON
        <textarea bind:value={params} rows="8" spellcheck="false"></textarea>
      </label>
    </div>

    <div class="cli-actions">
      <button class="btn btn--solid" onclick={() => void run()} disabled={busy || !method.trim()}>
        {busy ? "Esecuzione…" : "Esegui"}
      </button>
      <span class="hint">Ctrl/Cmd+Invio per eseguire</span>
    </div>

    {#if error}
      <p class="import-error">{error}</p>
    {/if}

    {#if result}
      <section class="cli-output">
        <div class="cli-output__head">
          <strong>{result.ok ? "Completato" : "Errore"}</strong>
          <span class="hint">{result.binary}</span>
        </div>
        {#if result.stderr.trim()}
          <pre class="cli-stderr">{result.stderr}</pre>
        {/if}
        <pre>{output}</pre>
      </section>
    {/if}
  </div>
</div>

<style>
  .cli-route {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 8px;
    margin: 10px 0 14px;
  }
  .cli-route label {
    flex: 1 1 280px;
    display: grid;
    gap: 4px;
    font-size: 0.82rem;
    font-weight: 600;
  }
  .cli-route input {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    padding: 8px 10px;
    font: inherit;
    font-weight: 400;
  }
</style>
