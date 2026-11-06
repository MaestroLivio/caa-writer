<script lang="ts">
  import type { Profile } from "../types";

  interface Props {
    onSave: (profile: Profile, pin: string | null) => Promise<void>;
    onCancel?: () => void;
  }

  let { onSave, onCancel }: Props = $props();

  const allFeatures = [
    "boards",
    "editor",
    "speech",
    "compose",
    "search",
    "save",
    "print",
    "agenda",
    "social",
    "didactic",
    "online",
    "ai",
  ];
  const labels: Record<string, string> = {
    boards: "Tabelle comunicative",
    editor: "Editor libero",
    speech: "Sintesi vocale",
    compose: "Componi frase",
    search: "Ricerca pittogrammi",
    save: "Salvataggio personale",
    print: "Stampa",
    agenda: "Agenda",
    social: "Storie sociali",
    didactic: "Attività didattiche",
    online: "Ricerca online",
    ai: "Strumenti IA",
  };

  let name = $state("");
  let ageTheme = $state("primaria");
  let editorLevel = $state(2);
  let features = $state<string[]>(["boards", "speech", "compose", "search", "save"]);
  let pin = $state("");
  let busy = $state(false);
  let error = $state("");

  function toggle(feature: string) {
    features = features.includes(feature)
      ? features.filter((item) => item !== feature)
      : [...features, feature];
  }

  async function save() {
    if (!name.trim() || busy) {
      error = "Inserisci un nome o uno pseudonimo.";
      return;
    }
    busy = true;
    error = "";
    const id = `profile-${Date.now().toString(36)}`;
    try {
      await onSave(
        {
          id,
          name: name.trim(),
          avatar: "",
          age_theme: ageTheme,
          editor_level: editorLevel,
          features,
          voice: {},
          accessibility: {},
          navigation: { mode: features.includes("compose") ? "compose" : "touch" },
          created_at: "",
          updated_at: "",
        },
        pin.trim() ? pin.trim() : null
      );
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="dialog-backdrop">
  <div class="report-dialog profile-setup" role="dialog" aria-modal="true" aria-label="Configura il profilo">
    <header class="report-dialog__head">
      <h2>Configura il primo profilo</h2>
      {#if onCancel}<button class="btn btn--ghost" onclick={onCancel} aria-label="Chiudi">×</button>{/if}
    </header>
    <p class="hint">
      Il profilo contiene solo preferenze operative. Non servono diagnosi o dati sanitari.
    </p>

    <label class="profile-field">
      Nome o pseudonimo
      <input bind:value={name} placeholder="Es. Luca" />
    </label>

    <div class="profile-row">
      <label class="profile-field">
        Tema grafico
        <select bind:value={ageTheme}>
          <option value="infanzia">Infanzia</option>
          <option value="primaria">Scuola primaria</option>
          <option value="secondaria">Scuola secondaria</option>
          <option value="adulto">Adulto</option>
          <option value="contrasto">Alto contrasto</option>
          <option value="minimal">Minimalista</option>
        </select>
      </label>
      <label class="profile-field">
        Livello editor
        <select bind:value={editorLevel}>
          <option value={1}>1 · Essenziale</option>
          <option value={2}>2 · Guidato</option>
          <option value={3}>3 · Avanzato</option>
        </select>
      </label>
    </div>

    <fieldset class="profile-features">
      <legend>Funzionalità disponibili nella modalità Utilizzatore</legend>
      {#each allFeatures as feature (feature)}
        <label>
          <input type="checkbox" checked={features.includes(feature)} onchange={() => toggle(feature)} />
          {labels[feature] ?? feature}
        </label>
      {/each}
    </fieldset>

    <label class="profile-field">
      PIN opzionale per l'Area Creator (minimo 4 caratteri)
      <input type="password" bind:value={pin} placeholder="Lascia vuoto per nessun PIN" />
    </label>

    {#if error}<p class="import-error">{error}</p>{/if}

    <div class="profile-actions">
      <button class="btn btn--solid" onclick={() => void save()} disabled={busy}>
        {busy ? "Creazione…" : "Crea profilo"}
      </button>
    </div>
  </div>
</div>
