<script lang="ts">
  // Barra di avanzamento globale dei download.
  //
  // Più pacchetti possono essere scaricati insieme: questa finestra somma i
  // byte di tutti i job e mostra l'avanzamento di ciascuno. Quando è
  // obbligatoria non si chiude finché i download non sono terminati; in ogni
  // caso avvisa di non chiudere la finestra del programma.
  import { onDestroy, onMount } from "svelte";
  import { api, isTauri } from "../api";
  import type { DownloadSnapshot } from "../types";

  interface Props {
    mandatory?: boolean;
    /// Chiamata una sola volta quando tutti i job sono terminati.
    onFinished?: () => void;
    onClose?: () => void;
  }

  let { mandatory = false, onFinished, onClose }: Props = $props();

  let snapshot = $state<DownloadSnapshot | null>(null);
  let error = $state("");
  let notified = false;
  let poll: ReturnType<typeof setInterval> | null = null;

  async function refresh() {
    if (!isTauri()) return;
    try {
      snapshot = await api.downloadStatus();
      if (snapshot.tasks.length > 0 && !snapshot.active && !notified) {
        notified = true;
        onFinished?.();
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
    poll = setInterval(() => void refresh(), 600);
    return () => {
      if (poll) clearInterval(poll);
      poll = null;
    };
  });

  onDestroy(() => {
    if (poll) clearInterval(poll);
  });

  function mb(bytes: number): string {
    if (!bytes) return "0 MB";
    return `${(bytes / 1_048_576).toFixed(1)} MB`;
  }

  const failed = $derived(snapshot?.tasks.filter((task) => task.error) ?? []);
  const allDone = $derived(Boolean(snapshot && snapshot.tasks.length > 0 && !snapshot.active));
  const canClose = $derived(!mandatory || allDone || failed.length > 0 || snapshot?.tasks.length === 0);
</script>

<div class="dl" role="dialog" aria-modal={mandatory ? "true" : undefined} aria-labelledby="dl-title">
  <div class="dl__card">
    <header class="dl__head">
      <div>
        <h2 id="dl-title">Download dei pacchetti</h2>
        <p>
          Puoi avviare più download contemporaneamente. <strong>Non chiudere la finestra</strong>
          finché la barra non arriva al 100%: i file vengono salvati nella cartella dati
          dell'utente e restano disponibili offline.
        </p>
      </div>
      <button
        class="btn btn--ghost"
        disabled={!canClose}
        onclick={() => onClose?.()}
        aria-label="Chiudi"
        title={canClose ? "Chiudi" : "Attendi la fine dei download"}
      >×</button>
    </header>

    {#if error}
      <p class="dl__error" role="alert">{error}</p>
    {/if}

    <div class="dl__global">
      <div class="dl__bar" aria-hidden="true">
        <span style={`width:${snapshot?.percent ?? 0}%`}></span>
      </div>
      <p>
        {#if snapshot}
          <strong>{Math.round(snapshot.percent)}%</strong>
          · {mb(snapshot.done)} / {mb(snapshot.total)}
          {#if snapshot.active}· download in corso{:else if snapshot.tasks.length}· completato{/if}
        {:else}
          Lettura dello stato…
        {/if}
      </p>
    </div>

    <div class="dl__tasks">
      {#each snapshot?.tasks ?? [] as task (task.id)}
        <article class="dl__task" class:dl__task--error={task.error}>
          <div class="dl__task-head">
            <strong>{task.label}</strong>
            <span>{task.error ? "Errore" : task.finished ? "Completato" : `${Math.round(task.percent)}%`}</span>
          </div>
          <div class="dl__bar dl__bar--task" aria-hidden="true">
            <span style={`width:${task.error ? 100 : task.percent}%`}></span>
          </div>
          <p>{task.error ?? task.phase}</p>
          {#if task.error}
            <button
              class="btn"
              onclick={() => {
                error = "";
                void api.downloadStart(task.id).then(refresh).catch((err) => {
                  error = err instanceof Error ? err.message : String(err);
                });
              }}
            >Riprova</button>
          {/if}
        </article>
      {/each}
      {#if snapshot && snapshot.tasks.length === 0}
        <p class="dl__empty">Nessun download in corso.</p>
      {/if}
    </div>

    <footer class="dl__footer">
      <span class="dl__hint">
        {#if allDone}
          Tutti i download sono terminati. Puoi continuare a usare il programma offline.
        {:else}
          Puoi continuare a usare le funzioni già installate, ma non chiudere la finestra.
        {/if}
      </span>
      {#if allDone || failed.length > 0}
        <button class="btn btn--solid" onclick={() => onClose?.()}>
          {failed.length ? "Continua offline" : "Continua"}
        </button>
      {/if}
    </footer>
  </div>
</div>

<style>
  .dl {
    position: fixed;
    inset: 0;
    z-index: 220;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(15, 23, 42, 0.52);
    overflow: auto;
  }
  .dl__card {
    width: min(760px, 100%);
    max-height: min(90vh, 820px);
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    background: #fff;
    color: #0f172a;
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(15, 23, 42, 0.32);
    overflow: auto;
  }
  .dl__head {
    display: flex;
    justify-content: space-between;
    gap: 14px;
  }
  .dl__head h2 { margin: 0; font-size: 1.15rem; }
  .dl__head p { margin: 5px 0 0; max-width: 620px; color: #475569; font-size: 0.84rem; line-height: 1.45; }
  .dl__global { display: grid; gap: 4px; }
  .dl__global p { margin: 0; font-size: 0.9rem; }
  .dl__bar {
    height: 16px;
    border-radius: 999px;
    background: #e2e8f0;
    overflow: hidden;
  }
  .dl__bar span {
    display: block;
    height: 100%;
    background: linear-gradient(90deg, #0f766e, #2563eb);
    transition: width 220ms ease;
  }
  .dl__bar--task { height: 9px; }
  .dl__tasks { display: grid; gap: 8px; }
  .dl__task {
    display: grid;
    gap: 5px;
    padding: 10px 12px;
    border: 1px solid #dbe3ee;
    border-radius: 10px;
    background: #f8fafc;
  }
  .dl__task--error { border-color: #fecaca; background: #fef2f2; }
  .dl__task-head { display: flex; justify-content: space-between; gap: 10px; font-size: 0.84rem; }
  .dl__task p { margin: 0; color: #475569; font-size: 0.78rem; }
  .dl__empty { color: #64748b; font-size: 0.84rem; }
  .dl__footer { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .dl__hint { margin-right: auto; color: #475569; font-size: 0.8rem; }
  .dl__error { margin: 0; padding: 8px 10px; border-radius: 8px; background: #fef2f2; color: #991b1b; font-size: 0.82rem; }
  .btn {
    border: 1px solid #cbd5e1;
    background: #fff;
    color: #1e293b;
    border-radius: 8px;
    padding: 8px 12px;
    font: inherit;
    font-size: 0.84rem;
    cursor: pointer;
  }
  .btn--solid { background: #0f766e; border-color: #0f766e; color: #fff; font-weight: 700; }
  .btn--ghost { border-color: transparent; background: transparent; font-size: 1.3rem; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
