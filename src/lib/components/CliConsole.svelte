<script lang="ts">
  import { api } from "../api";
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

  let method = $state("info");
  let params = $state("{}");
  let busy = $state(false);
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
