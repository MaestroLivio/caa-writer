<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { api, isTauri } from "../api";
  import type { AiStatus, BenchmarkResult, InstallStatus, ModelEntry } from "../types";

  interface Props {
    aiStatus: AiStatus | null;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
    /// Chiamata dopo un cambio reale (modello attivo, download concluso):
    /// ricarica lo stato IA dell'applicazione.
    onStatusChange: () => Promise<void>;
  }

  let { aiStatus, onToast, onStatusChange }: Props = $props();

  let models = $state<ModelEntry[]>([]);
  let installStatus = $state<InstallStatus | null>(null);
  let benchmark = $state<BenchmarkResult | null>(null);
  let benchBusy = $state(false);
  let importBusy = $state(false);
  // Modello di cui *questa* schermata ha avviato il download: solo quello
  // può essere attivato automaticamente al termine. Un download concluso in
  // una sessione precedente non deve sovrascrivere la scelta manuale.
  let pendingModelId = "";
  let activatedId = "";
  let lastMessage = "";
  let poll: ReturnType<typeof setInterval> | null = null;

  const hardware = $derived(aiStatus?.hardware ?? null);
  const metrics = $derived(
    aiStatus?.metrics ?? {
      requests: 0,
      completed: 0,
      cache_hits: 0,
      cancelled: 0,
      failures: 0,
      total_ms: 0,
      last_ms: 0,
      executor: "",
    },
  );
  const activePath = $derived(aiStatus?.model_path ?? "");
  // Il modello più capace che entra nella RAM disponibile: è un suggerimento,
  // non un'attivazione automatica.
  const suggested = $derived.by(() => {
    const ram = hardware?.available_ram_mb ?? hardware?.total_ram_mb ?? 0;
    const fitting = models
      .filter((model) => ram === 0 || model.minimum_ram_mb <= ram)
      .sort((a, b) => a.minimum_ram_mb - b.minimum_ram_mb);
    return fitting.length > 0 ? fitting[fitting.length - 1] : undefined;
  });

  function statusBusy(status: InstallStatus | null): boolean {
    return status?.phase === "downloading" || status?.phase === "verifying";
  }

  onMount(() => {
    if (!isTauri()) return;
    void refreshModels();
    void refreshStatus();
  });

  onDestroy(() => stopPolling());

  function stopPolling() {
    if (poll) {
      clearInterval(poll);
      poll = null;
    }
  }

  function startPolling() {
    if (poll) return;
    poll = setInterval(() => void refreshStatus(), 700);
  }

  async function refreshModels() {
    try {
      models = await api.aiModels();
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function refreshStatus() {
    try {
      const status = await api.aiInstallStatus();
      installStatus = status;
      if (statusBusy(status)) {
        startPolling();
        return;
      }
      stopPolling();
      if (status.message && status.message !== lastMessage) {
        lastMessage = status.message;
        if (status.phase === "error") onToast(status.message, "error");
        if (status.phase === "cancelled") onToast(status.message, "warn");
      }
      // Download concluso: si attiva solo il modello richiesto da questa
      // schermata, mai uno stato «ready» ereditato da una sessione precedente.
      if (
        status.phase === "ready" &&
        status.model_id &&
        status.model_id === pendingModelId &&
        status.model_id !== activatedId
      ) {
        activatedId = status.model_id;
        pendingModelId = "";
        const entry = models.find((model) => model.id === status.model_id);
        try {
          await api.aiSelectModel(status.model_id);
          await refreshModels();
          await onStatusChange();
          onToast(`«${entry?.name ?? status.model_id}» installato e attivo.`, "info");
        } catch (error) {
          onToast(error instanceof Error ? error.message : String(error), "error");
        }
      }
    } catch {
      stopPolling();
    }
  }

  async function install(model: ModelEntry) {
    if (!isTauri()) return;
    try {
      activatedId = "";
      pendingModelId = model.id;
      installStatus = await api.aiInstallModel(model.id);
      startPolling();
      onToast(`Download di «${model.name}» avviato.`, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function cancelInstall() {
    try {
      pendingModelId = "";
      installStatus = await api.aiCancelModelInstall();
      stopPolling();
      onToast("Download annullato: riprenderà da dove si era fermato.", "warn");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function select(model: ModelEntry) {
    try {
      // Una scelta esplicita annulla l'attivazione automatica del download.
      pendingModelId = "";
      await api.aiSelectModel(model.id);
      await refreshModels();
      await onStatusChange();
      onToast(`Modello attivo: ${model.name}.`, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function remove(model: ModelEntry) {
    const confirmed = window.confirm(
      `Eliminare il file di «${model.name}»? Potrai scaricarlo di nuovo o reimportarlo.`
    );
    if (!confirmed) return;
    try {
      pendingModelId = "";
      await api.aiRemoveModel(model.id);
      await refreshModels();
      await onStatusChange();
      onToast(`«${model.name}» rimosso dal disco.`, "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  async function importModel() {
    if (!isTauri()) return;
    const chosen = await openDialog({
      multiple: false,
      directory: false,
      title: "Scegli un modello GGUF",
      filters: [{ name: "Modello GGUF", extensions: ["gguf"] }],
    });
    if (typeof chosen !== "string") return;
    importBusy = true;
    try {
      await api.aiImportModel(chosen);
      await refreshModels();
      await onStatusChange();
      onToast("Modello GGUF importato e attivato.", "info");
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      importBusy = false;
    }
  }

  async function runBenchmark() {
    benchBusy = true;
    benchmark = null;
    try {
      benchmark = await api.aiBenchmark();
    } catch (error) {
      onToast(error instanceof Error ? error.message : String(error), "error");
    } finally {
      benchBusy = false;
    }
  }

  function megabytes(bytes: number): string {
    return `${Math.round(bytes / 1048576)} MB`;
  }

  function gigabytes(bytes: number): string {
    return `${(bytes / 1073741824).toFixed(1)} GB`;
  }

  function fitsRam(model: ModelEntry): boolean {
    const ram = hardware?.available_ram_mb ?? hardware?.total_ram_mb ?? 0;
    return ram === 0 || ram >= model.minimum_ram_mb;
  }

  function isActive(model: ModelEntry): boolean {
    return Boolean(activePath) && Boolean(model.local_path) && model.local_path === activePath;
  }

  function phaseLabel(model: ModelEntry): string {
    const status = installStatus;
    if (status && status.model_id === model.id) {
      switch (status.phase) {
        case "downloading":
          return `download ${Math.round(status.percent)}% · ${megabytes(status.downloaded_bytes)} di ${megabytes(status.total_bytes)}`;
        case "verifying":
          return "verifica dell'impronta sha256…";
        case "error":
          return "download non riuscito";
        case "cancelled":
          return "download annullato (riprendibile)";
        case "ready":
          return "installato";
        default:
          break;
      }
    }
    if (model.bundled) return "incluso nell'applicazione";
    if (model.installed) return "installato";
    return "non installato";
  }
</script>

<section class="panel-section">
  <h3>Gestione modelli neurali</h3>
  {#if hardware}
    <dl class="detail-list">
      <dt>Profilo</dt><dd>{hardware.profile}</dd>
      <dt>RAM</dt>
      <dd>
        {Math.round(hardware.total_ram_mb / 1024)} GB totali, {Math.round(hardware.available_ram_mb / 1024)} GB liberi
      </dd>
      <dt>CPU</dt><dd>{hardware.cpu_cores} core · {hardware.arch}</dd>
      <dt>Accelerazione</dt><dd>{hardware.accelerated ? "disponibile" : "solo CPU"}</dd>
      <dt>Modello attivo</dt><dd>{aiStatus?.active_model ?? "nessuno"}</dd>
      <dt>Istanza condivisa</dt>
      <dd>{aiStatus?.server_running ? "attiva (llama-server)" : "non avviata"}</dd>
      <dt>Esecutore</dt><dd>{metrics.executor || "—"}</dd>
    </dl>
    <p class="hint">{hardware.note}</p>
    <dl class="detail-list">
      <dt>Richieste</dt><dd>{metrics.requests} ({metrics.completed} completate)</dd>
      <dt>Cache</dt><dd>{metrics.cache_hits} risposte riutilizzate</dd>
      <dt>Annullate</dt><dd>{metrics.cancelled} · fallite {metrics.failures}</dd>
      <dt>Ultima latenza</dt><dd>{metrics.last_ms} ms</dd>
    </dl>
  {:else}
    <p class="hint">
      Dati hardware non disponibili in questa build (backend simulato): la gestione dei modelli
      richiede l'app desktop.
    </p>
  {/if}

  {#if models.length === 0}
    <p class="hint">Catalogo non disponibile in questa build.</p>
  {:else}
    {#if suggested && !suggested.bundled && !suggested.installed}
      <p class="hint">
        Con {Math.round((hardware?.available_ram_mb ?? 0) / 1024)} GB liberi, il modello più capace
        che entra in memoria è <strong>{suggested.name}</strong> ({gigabytes(suggested.size_bytes)} su disco).
        Il download è un'azione esplicita: nessun modello viene scaricato da solo.
      </p>
    {/if}
    <ul class="model-list">
      {#each models as model (model.id)}
        <li class="model-card" class:model-card--active={isActive(model)}>
          <div class="model-card__head">
            <strong>{model.name}</strong>
            {#if isActive(model)}
              <span class="badge badge--ok">attivo</span>
            {/if}
            {#if model.bundled}
              <span class="badge">incluso</span>
            {:else if model.installed}
              <span class="badge">installato</span>
            {/if}
            {#if suggested?.id === model.id && !isActive(model)}
              <span class="badge badge--warn">consigliato per questa macchina</span>
            {/if}
          </div>
          <div class="hint">
            {model.parameters} · {model.quantization} · {megabytes(model.size_bytes)} ·
            contesto {model.context_length} token · RAM minima {Math.round(model.minimum_ram_mb / 1024)} GB
          </div>
          <div class="hint">
            Licenza {model.license} · {phaseLabel(model)}
          </div>
          {#if !fitsRam(model)}
            <p class="warn">
              La memoria libera ({Math.round((hardware?.available_ram_mb ?? 0) / 1024)} GB) è sotto il
              minimo dichiarato: il modello può non avviarsi. Puoi comunque scaricarlo e provarlo.
            </p>
          {/if}
          {#if installStatus && installStatus.model_id === model.id && statusBusy(installStatus)}
            <progress max="100" value={installStatus.percent}></progress>
            <p class="hint">{installStatus.message}</p>
          {:else if installStatus && installStatus.model_id === model.id && installStatus.phase === "error"}
            <p class="warn">{installStatus.message}</p>
          {/if}
          <p class="hint">{model.notes}</p>
          {#if model.sha256}
            <p class="hint model-hash">sha256 {model.sha256.slice(0, 16)}… (verificata dopo il download)</p>
          {/if}
          <div class="model-card__actions">
            {#if installStatus && installStatus.model_id === model.id && statusBusy(installStatus)}
              <button class="btn btn--ghost" onclick={() => void cancelInstall()}>Annulla download</button>
            {:else if model.installed && !isActive(model)}
              <button class="btn btn--solid" onclick={() => void select(model)}>Usa questo modello</button>
            {:else if !model.installed && !model.bundled && model.verified && model.download_url}
              <button class="btn btn--solid" onclick={() => void install(model)}>
                Scarica e usa ({gigabytes(model.size_bytes)})
              </button>
            {/if}
            {#if model.installed && !model.bundled}
              <button class="btn btn--ghost" onclick={() => void remove(model)}>Rimuovi dal disco</button>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="model-actions">
    <button class="btn btn--ghost" disabled={importBusy} onclick={() => void importModel()}>
      {importBusy ? "Importazione…" : "Importa un modello GGUF…"}
    </button>
    <button class="btn btn--ghost" disabled={benchBusy} onclick={() => void runBenchmark()}>
      {benchBusy ? "Misura in corso…" : "Misura le prestazioni reali"}
    </button>
  </div>
  <p class="hint">
    L'importazione manuale serve per un GGUF già presente sul computer: viene verificata la firma
    del file e copiato nella cartella dei modelli dell'app. Nessun file viene inviato altrove.
  </p>
  {#if benchmark}
    <p class="hint">
      {benchmark.runs} prove · media {benchmark.average_ms} ms · {benchmark.failures} fallite.
      {benchmark.note}
    </p>
  {/if}
</section>

<style>
  .model-list {
    list-style: none;
    margin: 10px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .model-card {
    border: 1px solid rgba(100, 116, 139, 0.28);
    border-radius: 10px;
    padding: 10px;
    font-size: 0.82rem;
  }
  .model-card--active {
    border-color: var(--accent, #2563eb);
    box-shadow: 0 0 0 1px var(--accent, #2563eb);
  }
  .model-card__head {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
  }
  .model-card__actions,
  .model-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 8px;
  }
  .badge {
    font-size: 0.68rem;
    padding: 1px 6px;
    border-radius: 999px;
    border: 1px solid rgba(100, 116, 139, 0.4);
  }
  .badge--ok {
    border-color: #15803d;
    color: #15803d;
  }
  .badge--warn {
    border-color: #b45309;
    color: #b45309;
  }
  .hint {
    font-size: 0.78rem;
    opacity: 0.78;
    margin: 4px 0;
  }
  .warn {
    font-size: 0.78rem;
    color: #b45309;
    margin: 4px 0;
  }
  .model-hash {
    font-family: ui-monospace, monospace;
    font-size: 0.7rem;
  }
  progress {
    width: 100%;
    height: 8px;
    margin-top: 6px;
  }
</style>
