<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import ModelManager from "./ModelManager.svelte";
  import type {
    AiStatus,
    LibraryPackageState,
    LibraryPackagesStatus,
    PackageCatalog,
    PackageInfo,
    PackageOutline,
    UpdateInfo,
    VoiceStatus,
  } from "../types";

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
  // Stato dei pacchetti della libreria: versione installata, inclusa e
  // pubblicata nel repository. L'aggiornamento è per singolo pacchetto.
  let libraryStatus = $state<LibraryPackagesStatus | null>(null);
  let libraryError = $state("");
  let libraryBusy = $state("");
  let libraryReport = $state("");
  let outlines = $state<Record<string, PackageOutline | "loading" | "error">>({});

  onMount(() => {
    void load();
    void loadVoice();
    void loadLibraryStatus(false);
  });

  async function loadLibraryStatus(checkRemote: boolean) {
    libraryError = "";
    try {
      libraryStatus = await api.libraryPackagesStatus(checkRemote);
    } catch (err) {
      libraryError = err instanceof Error ? err.message : String(err);
    }
  }

  async function updatePackage(item: LibraryPackageState) {
    libraryBusy = item.id;
    libraryError = "";
    libraryReport = "";
    try {
      const source = item.action === "repository" ? "repository" : "inclusa";
      const result = await api.libraryPackageUpdate(item.id, source, true);
      libraryReport = result.message;
      await loadLibraryStatus(false);
      onChanged?.();
    } catch (err) {
      libraryError = err instanceof Error ? err.message : String(err);
    } finally {
      libraryBusy = "";
    }
  }

  async function updateAllPackages() {
    libraryBusy = "*";
    libraryError = "";
    libraryReport = "";
    try {
      const report = await api.libraryPackagesUpdateAll("auto", true);
      libraryReport =
        report.updated === 0
          ? "Nessun pacchetto da aggiornare."
          : `${report.updated} pacchetti aggiornati${report.failed ? `, ${report.failed} non riusciti` : ""}.`;
      for (const warning of report.warnings) {
        libraryError = libraryError ? `${libraryError} ${warning}` : warning;
      }
      await loadLibraryStatus(false);
      onChanged?.();
    } catch (err) {
      libraryError = err instanceof Error ? err.message : String(err);
    } finally {
      libraryBusy = "";
    }
  }

  async function toggleOutline(id: string) {
    if (outlines[id] && outlines[id] !== "error") {
      const next = { ...outlines };
      delete next[id];
      outlines = next;
      return;
    }
    outlines = { ...outlines, [id]: "loading" };
    try {
      const outline = await api.libraryPackageOutline(id);
      outlines = { ...outlines, [id]: outline };
    } catch {
      outlines = { ...outlines, [id]: "error" };
    }
  }

  function packageId(item: PackageInfo): string {
    return item.id.replace(/^library:/, "");
  }

  function statusFor(item: PackageInfo): LibraryPackageState | undefined {
    return libraryStatus?.packages.find((pkg) => pkg.id === packageId(item));
  }

  function outlineFor(item: PackageInfo): PackageOutline | null {
    const value = outlines[packageId(item)];
    return value && value !== "loading" && value !== "error" ? value : null;
  }

  function formatDate(value: string | null | undefined): string {
    if (!value) return "data non disponibile";
    const parsed = new Date(value);
    if (Number.isNaN(parsed.getTime())) return value;
    return parsed.toLocaleDateString("it-IT");
  }

  function formatMb(bytes: number): string {
    return bytes > 0 ? `${(bytes / 1_048_576).toFixed(1)} MB` : "";
  }

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
      Scelti al primo avvio: {requested.map((id) => id === "ai-model" ? "modello neurale locale" : id === "library" ? "libreria materiali" : id === "voice" ? "voce neurale" :id === "arasaac" ? "pittogrammi ARASAAC" : "Global Symbols").join(", ")}.
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
            <button
              class="btn btn--solid"
              disabled={!onStartDownload}
              title={onStartDownload ? undefined : "Il download automatico richiede l'app desktop."}
              onclick={() => onStartDownload?.(["update"])}
            >Aggiorna ora</button>
            <span class="hint">
              Il download sceglie automaticamente il pacchetto per questo computer e apre
              l'installatore: su macOS il DMG, su Windows il programma di installazione. La
              libreria materiali è inclusa e si aggiorna con l'app.
            </span>
          {:else if updates.latest_version}
            <span class="badge">Già aggiornato ({updates.latest_version})</span>
          {:else}
            <span class="badge">Versione più recente non letta</span>
          {/if}
        </div>
        <div class="updates__row">
          <strong>Libreria materiali</strong>
          <span>
            {#if libraryStatus}
              {libraryStatus.packages.length} pacchetti ·
              {libraryStatus.outdated === 0
                ? "tutti aggiornati"
                : `${libraryStatus.outdated} da aggiornare`}
            {:else}
              stato dei pacchetti non letto
            {/if}
          </span>
          {#if libraryStatus && libraryStatus.outdated > 0}
            <span class="badge badge--accent">
              {libraryStatus.outdated} aggiornamenti disponibili
            </span>
            <button
              class="btn btn--solid"
              disabled={libraryBusy === "*"}
              onclick={() => void updateAllPackages()}
            >{libraryBusy === "*" ? "Aggiornamento…" : "Aggiorna tutti i pacchetti"}</button>
          {:else if libraryStatus}
            <span class="badge">Libreria aggiornata</span>
          {/if}
          <span class="hint">
            La libreria si aggiorna per singolo pacchetto, senza reinstallare l'app: si apre la
            sezione «Pacchetti della libreria» e si sceglie il pacchetto da aggiornare.
          </span>
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
          Ogni categoria è un pacchetto con la sua versione: l'app dice quale versione è inclusa,
          quale è installata nel tuo profilo e quale è pubblicata nel repository. L'aggiornamento
          riguarda un pacchetto alla volta, funziona anche offline (dalla copia inclusa) e non tocca
          gli altri materiali.
        </p>
      </div>
      {#if libraryStatus && libraryStatus.outdated > 0}
        <button
          class="btn btn--solid"
          disabled={libraryBusy === "*"}
          onclick={() => void updateAllPackages()}
        >{libraryBusy === "*" ? "Aggiornamento…" : `Aggiorna tutti (${libraryStatus.outdated})`}</button>
      {/if}
    </div>
    {#if libraryError}
      <p class="notice notice--error" role="alert">{libraryError}</p>
    {/if}
    {#if libraryReport}
      <p class="notice notice--ok" role="status" aria-live="polite">{libraryReport}</p>
    {/if}
    {#if libraryStatus?.warnings.length}
      {#each libraryStatus.warnings as warning}
        <p class="updates__warn">{warning}</p>
      {/each}
    {/if}
    {#if loading && !catalog}
      <p class="notice">Caricamento dei pacchetti…</p>
    {:else if catalog}
      <div class="grid">
        {#each catalog.library as item (item.id)}
          {@const state = statusFor(item)}
          {@const outline = outlineFor(item)}
          {@const outdated = state?.status === "aggiornabile"}
          <article class="card" class:card--outdated={outdated}>
            <div class="card__head">
              <h4>{item.name}</h4>
              <span
                class="badge"
                class:badge--accent={!outdated && item.installed}
                class:badge--warn={outdated}
              >
                {outdated ? "Aggiornamento disponibile" : item.installed ? "In libreria" : "Rimosso"}
              </span>
            </div>
            <p>{item.description || "Materiali pronti da aprire e modificare."}</p>
            <div class="card__meta">
              <span>{item.count} materiali</span>
              {#if state}<span>inclusa {state.bundled_version}</span>{/if}
              {#if state?.installed_version}
                <span>installata {state.installed_version}</span>
              {:else if state}
                <span>copia inclusa</span>
              {/if}
              {#if state?.remote_version}<span>repository {state.remote_version}</span>{/if}
              {#if state && state.bytes > 0}<span>{formatMb(state.bytes)}</span>{/if}
              {#if item.license}<span>{item.license}</span>{/if}
            </div>
            <div class="card__actions">
              {#if outdated && state}
                <button
                  class="btn btn--solid"
                  disabled={libraryBusy === item.id}
                  aria-label={`Aggiorna il pacchetto ${item.name} alla versione ${state.action === "repository" ? state.remote_version ?? "" : state.bundled_version}`}
                  onclick={() => void updatePackage(state)}
                >
                  {libraryBusy === item.id
                    ? "Aggiornamento…"
                    : state.action === "repository"
                      ? "Aggiorna dal repository"
                      : "Aggiorna pacchetto"}
                </button>
              {/if}
              <button
                class="btn"
                aria-expanded={Boolean(outline) || outlines[packageId(item)] === "loading"}
                aria-label={`Scheda didattica di ${item.name}`}
                onclick={() => void toggleOutline(packageId(item))}
              >{outline ? "Chiudi scheda" : "Scheda didattica"}</button>
              {#if item.installed}
                <button class="btn btn--danger" disabled={busy === item.id} onclick={() => void toggleLibrary(item, false)}>
                  Rimuovi dalla libreria
                </button>
              {:else}
                <button class="btn btn--solid" disabled={busy === item.id} onclick={() => void toggleLibrary(item, true)}>
                  Ripristina nella libreria
                </button>
              {/if}
            </div>
            {#if outlines[packageId(item)] === "loading"}
              <p class="hint" role="status">Lettura della scheda…</p>
            {:else if outlines[packageId(item)] === "error"}
              <p class="hint">Scheda didattica non disponibile per questo pacchetto.</p>
            {:else if outline}
              <div class="outline">
                <p class="outline__meta">
                  {outline.documents} documenti · {outline.pages} pagine · versione {outline.version}
                </p>
                {#if outline.objectives.length}
                  <p class="outline__label">Obiettivi dichiarati</p>
                  <ul>
                    {#each outline.objectives.slice(0, 6) as objective}
                      <li>{objective}</li>
                    {/each}
                  </ul>
                {/if}
                {#if outline.subcategories.length}
                  <p class="outline__label">Sottocategorie</p>
                  <p class="outline__chips">
                    {#each outline.subcategories as [name, count]}
                      <span class="chip">{name} ({count})</span>
                    {/each}
                  </p>
                {/if}
                {#if outline.audiences.length}
                  <p class="outline__label">Pubblico</p>
                  <p class="outline__chips">
                    {#each outline.audiences as audience}
                      <span class="chip">{audience}</span>
                    {/each}
                  </p>
                {/if}
              </div>
            {/if}
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
          La sintesi vocale funziona sempre con la voce di sistema. La voce neurale Piper,
          se installata, legge offline con licenza dichiarata. Il download è esplicito e
          avviene una sola volta; il pannello mostra lo stato reale.
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
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(310px, 1fr)); gap: var(--space-3, 12px); }
  .card {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-4, 14px);
    display: flex;
    flex-direction: column;
    gap: var(--space-2, 8px);
  }
  .card--outdated { border-color: var(--warn); box-shadow: inset 3px 0 0 var(--warn); }
  .card__head { display: flex; justify-content: space-between; gap: 10px; align-items: flex-start; }
  .card h4 { margin: 0; font-size: 15px; color: var(--ink); }
  .card p { margin: 0; color: var(--muted); font-size: 12.5px; line-height: 1.45; }
  .card__meta { display: flex; flex-wrap: wrap; gap: 6px; color: var(--muted); font-size: 11.5px; }
  .card__meta > span::after { content: "·"; margin-left: 6px; color: var(--border); }
  .card__meta > span:last-child::after { content: ""; }
  .card__actions { display: flex; flex-wrap: wrap; gap: var(--space-2, 8px); margin-top: auto; }
  .badge {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--ink);
    padding: 3px 9px;
    font-size: 11px;
    font-weight: 700;
    white-space: nowrap;
  }
  .badge--accent { background: var(--accent-soft); color: #103a8f; }
  .badge--warn { background: var(--warn-soft); color: var(--warn); }
  .switch { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; color: var(--muted); white-space: nowrap; }
  .updates { display: grid; gap: var(--space-2, 8px); }
  .updates__row {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 10px var(--space-3, 12px);
    font-size: 12.5px;
    color: var(--ink);
  }
  .updates__row strong { min-width: 140px; }
  .updates__notes { background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius); padding: 8px 12px; font-size: 12px; }
  .updates__notes pre { white-space: pre-wrap; font: inherit; color: var(--ink); }
  .updates__warn {
    margin: 6px 0 0;
    color: var(--warn);
    background: var(--warn-soft);
    border: 1px solid color-mix(in srgb, var(--warn) 35%, transparent);
    border-radius: var(--radius);
    padding: 8px 10px;
    font-size: 12px;
  }
  .notice {
    padding: var(--space-3, 12px);
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--ink);
    font-size: 13px;
  }
  .notice--error { border-color: var(--danger); color: var(--danger); background: #fdf3f2; }
  .notice--ok { border-color: var(--ok); color: var(--ok); background: #f1faf4; }
  .hint { font-size: 12px; color: var(--muted); }
  .outline {
    border-top: 1px dashed var(--border);
    padding-top: var(--space-2, 8px);
    display: grid;
    gap: 4px;
  }
  .outline__meta { font-size: 11.5px; color: var(--muted); }
  .outline__label { margin-top: 6px; font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; color: var(--muted); }
  .outline ul { margin: 0; padding-left: 18px; display: grid; gap: 2px; }
  .outline li { font-size: 12px; color: var(--ink); line-height: 1.4; }
  .outline__chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { border-radius: 999px; background: var(--surface-2); color: var(--ink); padding: 3px 9px; font-size: 11px; }
  .btn {
    border: 1px solid var(--border);
    background: var(--surface);
    color: var(--ink);
    border-radius: 10px;
    min-height: var(--touch-target, 44px);
    padding: 8px 12px;
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    transition: background var(--anim-hover, 120ms), border-color var(--anim-hover, 120ms);
  }
  .btn:hover { background: var(--surface-2); }
  .btn--solid { background: var(--accent); border-color: var(--accent); color: var(--accent-ink); font-weight: 700; }
  .btn--solid:hover { background: #1749b8; }
  .btn--danger { color: var(--danger); border-color: color-mix(in srgb, var(--danger) 35%, transparent); background: #fdf3f2; }
  .btn--ghost { border-color: transparent; background: transparent; font-size: 20px; min-height: 0; }
  .btn:disabled { opacity: 0.55; cursor: not-allowed; }
  .btn:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
  @media (prefers-reduced-motion: reduce) {
    .btn { transition: none; }
  }
</style>
