<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import ModelManager from "./ModelManager.svelte";
  import type { AiStatus, PackageInfo, PackageCatalog, UpdateInfo, VoiceStatus } from "../types";

  interface Props {
    onClose: () => void;
    /// Segnala che una categoria è stata attivata o sospesa: la libreria
    /// deve ricaricare conteggi e risultati.
    onChanged?: () => void;
    /// Pacchetti scelti nella finestra del primo avvio. Non avviano download:
    /// servono solo a mostrare all'utente cosa ha chiesto.
    requested?: string[];
    /// Stato del motore neurale: permette di installare il modello da qui.
    aiStatus?: AiStatus | null;
    onStatusChange?: () => Promise<void>;
    onToast?: (message: string, kind?: "info" | "warn" | "error") => void;
    /// Avvia i download nel gestore globale con barra di avanzamento.
    onStartDownload?: (ids: string[]) => void;
  }

  let {
    onClose,
    onChanged,
    requested = [],
    aiStatus = null,
    onStatusChange,
    onToast,
    onStartDownload,
  }: Props = $props();

  let catalog = $state<PackageCatalog | null>(null);
  let loading = $state(true);
  let error = $state("");
  let busy = $state("");
  let report = $state("");
  let updates = $state<UpdateInfo | null>(null);
  let checking = $state(false);
  let updateError = $state("");
  let pendingRemove = $state<string | null>(null);
  let voiceStatus = $state<VoiceStatus | null>(null);
  let voiceError = $state("");

  onMount(() => {
    void load();
    void loadVoice();
  });

  async function loadVoice() {
    try {
      voiceStatus = await api.voiceStatus();
      voiceError = "";
    } catch (err) {
      voiceError = err instanceof Error ? err.message : String(err);
    }
  }

  async function load() {
    loading = true;
    error = "";
    try {
      catalog = await api.packageCatalog();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function toggleLibrary(item: PackageInfo, enabled: boolean) {
    const id = item.id.replace(/^library:/, "");
    busy = item.id;
    error = "";
    try {
      await api.packageSetLibrary(id, enabled);
      item.installed = enabled;
      catalog = catalog ? { ...catalog, library: [...catalog.library] } : catalog;
      onChanged?.();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = "";
    }
  }

  async function download(item: PackageInfo) {
    error = "";
    report = "";
    if (onStartDownload) {
      onStartDownload([item.id]);
      report = `Download di «${item.name}» avviato. Segui la barra globale: non chiudere la finestra.`;
      return;
    }
    busy = item.id;
    try {
      const result = await api.packageDownload(item.id);
      report = result.message;
      await load();
      onChanged?.();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = "";
    }
  }

  async function remove(item: PackageInfo) {
    if (pendingRemove !== item.id) {
      pendingRemove = item.id;
      return;
    }
    pendingRemove = null;
    busy = item.id;
    error = "";
    report = "";
    try {
      await api.packageRemove(item.id);
      report = `«${item.name}» rimosso dalla libreria locale.`;
      await load();
      onChanged?.();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = "";
    }
  }

  async function check() {
    checking = true;
    updateError = "";
    updates = null;
    try {
      updates = await api.checkUpdates();
    } catch (err) {
      updateError = err instanceof Error ? err.message : String(err);
    } finally {
      checking = false;
    }
  }

  async function openLink(url: string | null) {
    if (!url) return;
    try {
      await api.openRepositoryLink(url);
    } catch (err) {
      updateError = err instanceof Error ? err.message : String(err);
    }
  }

  function formatDate(value: string | null | undefined): string {
    if (!value) return "data non disponibile";
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? value : parsed.toLocaleDateString("it-IT");
  }
</script>

<div class="packages" role="dialog" aria-modal="true" aria-labelledby="packages-title">
  <header class="packages__top">
    <div>
      <h2 id="packages-title">Pacchetti e aggiornamenti</h2>
      <p>
        La libreria dei materiali è inclusa nell'app e funziona offline. I pacchetti di
        pittogrammi si scaricano in locale dai rispettivi repertori, con autore, licenza e fonte.
      </p>
    </div>
    <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
  </header>

  {#if error}
    <p class="notice notice--error" role="alert">{error}</p>
  {/if}
  {#if requested.length}
    <p class="notice notice--ok" role="status">
      Scelti al primo avvio: {requested.map((id) => id === "ai-model" ? "modello neurale locale" : id === "library" ? "libreria materiali" : id === "voice" ? "voce neurale" : id === "arasaac" ? "pittogrammi ARASAAC" : "Global Symbols").join(", ")}.
      Nessun download è partito: usa i pulsanti delle singole sezioni.
    </p>
  {/if}
  {#if report}
    <p class="notice notice--ok" role="status">{report}</p>
  {/if}

  <section class="packages__section" aria-labelledby="updates-title">
    <div class="packages__heading">
      <div>
        <h3 id="updates-title">Aggiornamenti</h3>
        <p>Il repository è pubblico: il controllo confronta la versione dell'app e la libreria con quella pubblicata.</p>
      </div>
      <button class="btn btn--solid" disabled={checking} onclick={() => void check()}>
        {checking ? "Controllo…" : "Controlla aggiornamenti"}
      </button>
    </div>
    {#if updateError}
      <p class="notice notice--error" role="alert">{updateError}</p>
    {/if}
    {#if updates}
      <div class="updates">
        <div class="updates__row">
          <strong>Software</strong>
          <span>Versione installata: {updates.current_version}</span>
          {#if updates.software_available && updates.latest_version}
            <span class="badge badge--accent">Disponibile {updates.latest_version}</span>
            <button class="btn" onclick={() => void openLink(updates!.release_url)}>Pagina della release</button>
          {:else if updates.latest_version}
            <span class="badge">Già aggiornato ({updates.latest_version})</span>
          {:else}
            <span class="badge">Versione più recente non letta</span>
          {/if}
        </div>
        <div class="updates__row">
          <strong>Libreria materiali</strong>
          <span>Versione locale: {formatDate(updates.library_current)}</span>
          {#if updates.library_available && updates.library_latest}
            <span class="badge badge--accent">Aggiornamento disponibile ({formatDate(updates.library_latest)})</span>
            <button class="btn" onclick={() => void openLink("https://github.com/MaestroLivio/caa-writer/releases/latest")}>Scarica la nuova libreria</button>
          {:else if updates.library_latest}
            <span class="badge">Libreria aggiornata</span>
          {:else}
            <span class="badge">Controllo libreria non disponibile</span>
          {/if}
        </div>
        {#if updates.release_notes}
          <details class="updates__notes">
            <summary>Note della release {updates.latest_version ?? ""}</summary>
            <pre>{updates.release_notes}</pre>
          </details>
        {/if}
        {#each updates.warnings as warning}
          <p class="updates__warn">{warning}</p>
        {/each}
      </div>
    {/if}
  </section>

  <section class="packages__section" aria-labelledby="library-packages-title">
    <div class="packages__heading">
      <div>
        <h3 id="library-packages-title">Pacchetti della libreria</h3>
        <p>
          I materiali di base sono inclusi nell'app e funzionano offline. Le categorie opzionali
          possono essere aggiunte o nascoste; in un'installazione snella la libreria completa si
          scarica a parte.
        </p>
      </div>
    </div>
    {#if loading && !catalog}
      <p class="notice">Caricamento dei pacchetti…</p>
    {:else if catalog}
      <div class="grid">
        {#each catalog.library as item (item.id)}
          <article class="card">
            <div class="card__head">
              <h4>{item.name}</h4>
              <span class:badge--accent={item.installed} class="badge">{item.installed ? "In libreria" : "Rimosso"}</span>
            </div>
            <p>{item.description || "Materiali pronti da aprire e modificare."}</p>
            <div class="card__meta">
              <span>{item.count} materiali</span>
              <span>{item.bundled ? "incluso nell'app" : "pacchetto opzionale"}</span>
              <span>offline</span>
              {#if item.license}<span>{item.license}</span>{/if}
            </div>
            <div class="card__actions">
              {#if !item.bundled && !item.installed}
                <button
                  class="btn btn--solid"
                  onclick={() => void openLink(item.source_url)}
                >
                  Apri la pagina della release
                </button>
                <p class="hint">
                  Il download automatico della libreria non è ancora disponibile: la pagina della
                  release indica il pacchetto da scaricare e dove copiarlo.
                </p>
              {:else if item.installed}
                <button class="btn btn--danger" disabled={busy === item.id} onclick={() => void toggleLibrary(item, false)}>
                  Rimuovi dalla libreria
                </button>
              {:else}
                <button class="btn btn--solid" disabled={busy === item.id} onclick={() => void toggleLibrary(item, true)}>
                  Ripristina nella libreria
                </button>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>

  <section class="packages__section" aria-labelledby="symbol-packages-title">
    <div class="packages__heading">
      <div>
        <h3 id="symbol-packages-title">Pacchetti di pittogrammi</h3>
        <p>
          Lo scaricamento richiede la rete una volta sola: le immagini vengono salvate nella
          cartella dati dell'utente e restano disponibili offline. Nessun pittogramma di terzi è
          incluso nell'installer.
        </p>
      </div>
    </div>
    {#if catalog}
      <div class="grid">
        {#each catalog.symbols as item (item.id)}
          <article class="card">
            <div class="card__head">
              <h4>{item.name}</h4>
              <span class:badge--accent={item.installed} class="badge">{item.installed ? "In locale" : "Non scaricato"}</span>
            </div>
            <p>{item.description}</p>
            <div class="card__meta">
              <span>{item.count} simboli</span>
              {#if item.version}<span>v{item.version}</span>{/if}
              {#if item.license}<span>{item.license}</span>{/if}
            </div>
            <div class="card__actions">
              {#if item.id === "symbols:arasaac"}
                <button class="btn btn--solid" disabled={busy === item.id} onclick={() => void download(item)}>
                  {busy === item.id ? "Scaricamento…" : "Scarica immagini offline"}
                </button>
                {#if item.removable && item.count > 0}
                  <button class="btn btn--danger" disabled={busy === item.id} onclick={() => void remove(item)}>
                    {pendingRemove === item.id ? "Conferma rimozione" : "Rimuovi"}
                  </button>
                  {#if pendingRemove === item.id}
                    <button class="btn" onclick={() => (pendingRemove = null)}>Annulla</button>
                  {/if}
                {/if}
              {:else if item.id === "symbols:globalsymbols"}
                <button class="btn btn--solid" disabled={busy === item.id} onclick={() => void download(item)}>
                  {busy === item.id ? "Scaricamento…" : item.installed ? "Aggiorna copia locale" : "Scarica in locale"}
                </button>
                {#if item.installed}
                  <button class="btn btn--danger" disabled={busy === item.id} onclick={() => void remove(item)}>
                    {pendingRemove === item.id ? "Conferma rimozione" : "Rimuovi"}
                  </button>
                  {#if pendingRemove === item.id}
                    <button class="btn" onclick={() => (pendingRemove = null)}>Annulla</button>
                  {/if}
                {/if}
              {:else}
                <span class="hint">Incluso nell'app; non richiede download.</span>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else if !loading}
      <p class="notice">Catalogo dei pacchetti non disponibile.</p>
    {/if}
  </section>

  <section class="packages__section" aria-labelledby="neural-packages-title">
    <div class="packages__heading">
      <div>
        <h3 id="neural-packages-title">Modello neurale locale</h3>
        <p>
          Il modello GGUF non è incluso nell'installer: l'app funziona senza. Se lo scegli, si
          scarica nella cartella dati dell'utente e resta offline. Nessun download parte da solo.
        </p>
      </div>
    </div>
    <ModelManager
      {aiStatus}
      onToast={onToast ?? (() => {})}
      onStatusChange={onStatusChange ?? (async () => {})}
    />
  </section>

  <section class="packages__section" aria-labelledby="voice-packages-title">
    <div class="packages__heading">
      <div>
        <h3 id="voice-packages-title">Voci</h3>
        <p>
          La sintesi vocale funziona sempre con la voce di sistema. La voce neurale Piper, se
          presente, legge offline con licenza dichiarata. In questa versione non è disponibile un
          download automatico della voce: il pannello mostra lo stato reale.
        </p>
      </div>
    </div>
    {#if voiceError}
      <p class="notice notice--error" role="alert">{voiceError}</p>
    {:else if voiceStatus}
      <div class="updates">
        <div class="updates__row">
          <strong>Motore vocale</strong>
          <span>{voiceStatus.backend === "piper" ? "Voce neurale locale" : "Voce di sistema"}</span>
          {#if voiceStatus.installed}
            <span class="badge badge--accent">Installata</span>
            {#if voiceStatus.voice}<span>{voiceStatus.voice}</span>{/if}
            {#if voiceStatus.license}<span>{voiceStatus.license}</span>{/if}
          {:else}
            <span class="badge">Voce neurale non installata</span>
            <span>La lettura usa la voce di sistema.</span>
          {/if}
        </div>
        {#if voiceStatus.voices.length}
          <div class="updates__row">
            <strong>Voci su disco</strong>
            <span>{voiceStatus.voices.map((voice) => voice.name).join(", ")}</span>
          </div>
        {/if}
        <div class="updates__row">
          <strong>Installa la voce neurale</strong>
          <span>Motore Piper e voce italiana Serena, circa 60 MB.</span>
          <button
            class="btn btn--solid"
            onclick={() => {
              if (onStartDownload) onStartDownload(["voice"]);
              else void loadVoice();
            }}
          >{voiceStatus.installed ? "Aggiorna la voce" : "Scarica e installa la voce"}</button>
        </div>
        {#each voiceStatus.notes as note}
          <p class="hint">{note}</p>
        {/each}
      </div>
    {:else}
      <p class="notice">Stato della voce non disponibile.</p>
    {/if}
  </section>

  <section class="packages__section" aria-labelledby="graphics-packages-title">
    <div class="packages__heading">
      <div>
        <h3 id="graphics-packages-title">Grafica e caratteri</h3>
        <p>
          In questa versione non ci sono pacchetti grafici da scaricare. Sfondi, cornici e
          illustrazioni arrivano dai modelli della libreria; i caratteri di alta leggibilità,
          scuola e pregrafismo si installano nel sistema operativo e l'app li usa senza
          incorporarli. Le licenze sono elencate in Impostazioni → Tipografia.
        </p>
      </div>
    </div>
  </section>

  {#if catalog?.warnings.length}
    <section class="packages__section">
      {#each catalog.warnings as warning}
        <p class="updates__warn">{warning}</p>
      {/each}
    </section>
  {/if}
</div>

<style>
  .packages {
    position: fixed;
    inset: 0;
    z-index: 120;
    overflow: auto;
    padding: 24px clamp(14px, 4vw, 48px) 48px;
    background: #f6f8fb;
    color: #0f172a;
  }
  .packages__top {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
    padding: 8px 0 16px;
    border-bottom: 1px solid #dbe3ee;
  }
  .packages__top h2 { margin: 0; font-size: 24px; }
  .packages__top p { margin: 6px 0 0; max-width: 820px; color: #475569; font-size: 13px; line-height: 1.45; }
  .packages__section { margin-top: 22px; }
  .packages__heading { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; margin-bottom: 12px; }
  .packages__heading h3 { margin: 0; font-size: 18px; }
  .packages__heading p { margin: 4px 0 0; max-width: 820px; color: #64748b; font-size: 12.5px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 12px; }
  .card { background: #fff; border: 1px solid #dbe3ee; border-radius: 12px; padding: 14px; display: flex; flex-direction: column; gap: 8px; }
  .card__head { display: flex; justify-content: space-between; gap: 10px; align-items: flex-start; }
  .card h4 { margin: 0; font-size: 15px; }
  .card p { margin: 0; color: #475569; font-size: 12.5px; line-height: 1.45; }
  .card__meta { display: flex; flex-wrap: wrap; gap: 6px; color: #64748b; font-size: 11px; }
  .card__meta > span::after { content: "·"; margin-left: 6px; color: #cbd5e1; }
  .card__meta > span:last-child::after { content: ""; }
  .card__actions { display: flex; flex-wrap: wrap; gap: 8px; margin-top: auto; }
  .badge { display: inline-flex; align-items: center; border-radius: 999px; background: #e2e8f0; color: #334155; padding: 2px 8px; font-size: 10.5px; font-weight: 700; white-space: nowrap; }
  .badge--accent { background: #ccfbf1; color: #115e59; }
  .switch { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: #475569; white-space: nowrap; }
  .updates { display: grid; gap: 8px; }
  .updates__row { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; background: #fff; border: 1px solid #dbe3ee; border-radius: 10px; padding: 10px 12px; font-size: 12.5px; }
  .updates__row strong { min-width: 140px; }
  .updates__notes { background: #fff; border: 1px solid #dbe3ee; border-radius: 10px; padding: 8px 12px; font-size: 12px; }
  .updates__notes pre { white-space: pre-wrap; font: inherit; color: #334155; }
  .updates__warn { margin: 6px 0 0; color: #92400e; background: #fffbeb; border: 1px solid #fde68a; border-radius: 8px; padding: 8px 10px; font-size: 12px; }
  .notice { padding: 12px; border-radius: 9px; background: #fff; border: 1px solid #dbe3ee; color: #475569; font-size: 13px; }
  .notice--error { border-color: #fecaca; background: #fef2f2; color: #991b1b; }
  .notice--ok { border-color: #bbf7d0; background: #f0fdf4; color: #166534; }
  .hint { font-size: 11.5px; color: #64748b; }
  .btn { border: 1px solid #cbd5e1; background: #fff; color: #1e293b; border-radius: 8px; padding: 7px 11px; font: inherit; font-size: 12px; cursor: pointer; }
  .btn:hover { background: #f1f5f9; }
  .btn--solid { background: #0f766e; border-color: #0f766e; color: #fff; font-weight: 700; }
  .btn--solid:hover { background: #115e59; }
  .btn--danger { color: #991b1b; border-color: #fecaca; background: #fef2f2; }
  .btn--ghost { border-color: transparent; background: transparent; font-size: 20px; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
