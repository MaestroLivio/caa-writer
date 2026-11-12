<script lang="ts">
  import { api } from "../api";
  import type { LibraryEntry, Profile } from "../types";

  interface Props {
    profiles: Profile[];
    creatorPin: string;
    onClose: () => void;
    onChanged: () => Promise<void>;
  }

  let { profiles, creatorPin, onClose, onChanged }: Props = $props();

  const allFeatures = [
    "boards",
    "editor",
    "speech",
    "compose",
    "search",
    "edit_symbols",
    "save",
    "export",
    "print",
    "agenda",
    "calendar",
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
    edit_symbols: "Modifica pittogrammi",
    save: "Salvataggio personale",
    export: "Esportazione",
    print: "Stampa",
    agenda: "Agenda",
    calendar: "Calendario",
    social: "Storie sociali",
    didactic: "Attività didattiche",
    online: "Ricerca online",
    ai: "Strumenti IA",
  };

  let materials = $state<LibraryEntry[]>([]);
  let materialQuery = $state("");
  let selectedId = $state(profiles[0]?.id ?? "");
  let draft = $state<Profile | null>(profiles[0] ? { ...profiles[0] } : null);
  let assigned = $state<Record<string, boolean>>({});
  let pin = $state("");
  let pinConfirm = $state("");
  let savingPin = $state(false);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");

  $effect(() => {
    void (async () => {
      try {
        const library = await api.universalLibrary();
        materials = [...library.entries].sort((a, b) =>
          a.title.localeCompare(b.title, "it")
        );
        if (selectedId) await loadAssignments(selectedId);
      } catch (err) {
        error = err instanceof Error ? err.message : String(err);
      }
    })();
  });

  const filteredMaterials = $derived.by(() => {
    const query = materialQuery.trim().toLowerCase();
    if (!query) return materials;
    return materials.filter(
      (item) =>
        item.title.toLowerCase().includes(query) ||
        item.category.toLowerCase().includes(query) ||
        item.kind.toLowerCase().includes(query)
    );
  });

  const assignedCount = $derived(
    Object.values(assigned).filter(Boolean).length
  );

  /// Solo i materiali realmente attribuiti: l'elenco non mostra i 330
  /// documenti della libreria, che rendevano il pannello lunghissimo.
  const assignedMaterials = $derived(
    materials.filter((material) => Boolean(assigned[material.id]))
  );

  const addableMaterials = $derived(
    filteredMaterials.filter((material) => !assigned[material.id])
  );

  let addOpen = $state(false);

  async function loadAssignments(profileId: string) {
    assigned = {};
    const rows = await api.profileBoards(profileId);
    for (const row of rows) assigned[row.boardId] = row.enabled;
  }

  async function removeAssigned(material: LibraryEntry) {
    if (!draft) return;
    assigned = { ...assigned, [material.id]: false };
    try {
      await api.profileAssignBoard(
        draft.id,
        material.id,
        false,
        0,
        creatorPin || null
      );
      notice = `«${material.title}» rimosso dal profilo.`;
    } catch (err) {
      assigned = { ...assigned, [material.id]: true };
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function addAssigned(material: LibraryEntry) {
    assigned = { ...assigned, [material.id]: true };
    notice = `«${material.title}» aggiunto: salva il profilo per confermare.`;
  }

  async function select(id: string) {
    selectedId = id;
    const found = profiles.find((profile) => profile.id === id);
    draft = found ? { ...found } : null;
    try {
      await loadAssignments(id);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function newProfile() {
    selectedId = "";
    draft = {
      id: `profile-${Date.now().toString(36)}`,
      name: "Nuovo profilo",
      avatar: "",
      age_theme: "primaria",
      editor_level: 2,
      features: ["boards", "compose", "search", "save"],
      voice: {},
      accessibility: {},
      navigation: { mode: "compose" },
      created_at: "",
      updated_at: "",
    };
    assigned = {};
  }

  function toggleFeature(feature: string) {
    if (!draft) return;
    draft.features = draft.features.includes(feature)
      ? draft.features.filter((item) => item !== feature)
      : [...draft.features, feature];
    draft = { ...draft };
  }

  async function save() {
    if (!draft || busy) return;
    busy = true;
    error = "";
    notice = "";
    try {
      await api.profileSave(draft, creatorPin || null);
      const position = 0;
      for (const [index, material] of materials.entries()) {
        await api.profileAssignBoard(
          draft.id,
          material.id,
          Boolean(assigned[material.id]),
          index,
          creatorPin || null
        );
      }
      void position;
      if (!profiles.some((profile) => profile.id === draft!.id)) {
        await api.profileSetActive(draft.id);
      }
      await onChanged();
      notice = "Profilo salvato.";
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function remove() {
    if (!draft || !profiles.some((profile) => profile.id === draft!.id)) return;
    if (!window.confirm(`Eliminare il profilo «${draft.name}»? I materiali originali restano.`)) return;
    busy = true;
    try {
      await api.profileDelete(draft.id, creatorPin || null);
      await onChanged();
      const next = profiles.find((profile) => profile.id !== draft!.id);
      if (next) await select(next.id);
      else newProfile();
      notice = "Profilo eliminato.";
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function setPin() {
    if (savingPin) return;
    if (pin && pin.length < 4) {
      error = "Il PIN deve avere almeno 4 caratteri.";
      return;
    }
    if (pin !== pinConfirm) {
      error = "I due PIN non coincidono.";
      return;
    }
    savingPin = true;
    error = "";
    try {
      await api.creatorSetPin(pin.trim() ? pin.trim() : null, creatorPin || null);
      notice = pin.trim() ? "PIN aggiornato." : "PIN rimosso.";
      pin = "";
      pinConfirm = "";
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      savingPin = false;
    }
  }
</script>

<div class="dialog-backdrop">
  <div class="report-dialog profile-manager" role="dialog" aria-modal="true" aria-label="Gestione utenti">
    <header class="report-dialog__head">
      <h2>Gestione utenti</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
    </header>

    <div class="profile-manager__body">
      <aside class="profile-list">
        {#each profiles as profile (profile.id)}
          <button
            class="profile-list__item"
            class:active={selectedId === profile.id}
            onclick={() => void select(profile.id)}
          >{profile.name}</button>
        {/each}
        <button class="btn btn--ghost" onclick={newProfile}>+ Nuovo profilo</button>
      </aside>

      {#if draft}
        <section class="profile-editor">
          <div class="profile-row">
            <label class="profile-field">
              Nome o pseudonimo
              <input bind:value={draft.name} />
            </label>
            <label class="profile-field">
              Tema
              <select bind:value={draft.age_theme}>
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
              <select bind:value={draft.editor_level}>
                <option value={1}>1 · Essenziale</option>
                <option value={2}>2 · Guidato</option>
                <option value={3}>3 · Avanzato</option>
              </select>
            </label>
          </div>

          <fieldset class="profile-features">
            <legend>Funzionalità abilitate</legend>
            {#each allFeatures as feature (feature)}
              <label>
                <input
                  type="checkbox"
                  checked={draft.features.includes(feature)}
                  onchange={() => toggleFeature(feature)}
                />
                {labels[feature] ?? feature}
              </label>
            {/each}
          </fieldset>

          <fieldset class="profile-boards">
            <legend>Materiali assegnati ({assignedCount})</legend>
            <p class="hint">
              Il profilo vede nella propria schermata solo questi documenti. Puoi rimuoverli
              dall'elenco in qualsiasi momento; gli originali restano nella libreria.
            </p>
            {#if assignedMaterials.length === 0}
              <p class="hint">Nessun materiale assegnato a questo profilo.</p>
            {:else}
              <div class="profile-materials">
                {#each assignedMaterials as material (material.id)}
                  <div class="profile-material">
                    <span>{material.title}</span>
                    <span class="hint">{material.category || material.kind} · {material.pages} pagine</span>
                    <button
                      class="btn btn--ghost"
                      disabled={busy}
                      onclick={() => void removeAssigned(material)}
                      aria-label={`Rimuovi ${material.title} dal profilo`}
                    >Rimuovi</button>
                  </div>
                {/each}
              </div>
            {/if}
            <button class="btn btn--ghost" onclick={() => (addOpen = !addOpen)}>
              {addOpen ? "Chiudi l'elenco dei materiali" : "Aggiungi materiali"}
            </button>
            {#if addOpen}
              <input
                class="profile-search"
                bind:value={materialQuery}
                placeholder="Cerca nella libreria…"
                aria-label="Cerca materiali da aggiungere"
              />
              <div class="profile-materials">
                {#each addableMaterials as material (material.id)}
                  <div class="profile-material">
                    <span>{material.title}</span>
                    <span class="hint">{material.category || material.kind} · {material.pages} pagine</span>
                    <button class="btn" onclick={() => addAssigned(material)}>Aggiungi</button>
                  </div>
                {/each}
                {#if addableMaterials.length === 0}
                  <p class="hint">Tutti i materiali trovati sono già assegnati.</p>
                {/if}
              </div>
            {/if}
          </fieldset>

          <div class="profile-actions">
            <button class="btn btn--solid" onclick={() => void save()} disabled={busy}>Salva profilo</button>
            <button class="btn btn--ghost" onclick={() => void remove()} disabled={busy || !selectedId}>Elimina</button>
            <button
              class="btn btn--ghost"
              onclick={() => void api.profileSetActive(draft!.id).then(onChanged)}
              disabled={busy || !selectedId}
            >Imposta come predefinito</button>
          </div>
        </section>
      {/if}
    </div>

    <section class="profile-pin">
      <h3>Protezione Area Creator</h3>
      <p class="hint">Il PIN protegge le operazioni amministrative e l'accesso all'Area Creator.</p>
      <div class="profile-row">
        <label class="profile-field">
          Nuovo PIN (vuoto = rimuovi)
          <input type="password" bind:value={pin} />
        </label>
        <label class="profile-field">
          Conferma PIN
          <input type="password" bind:value={pinConfirm} />
        </label>
        <button class="btn" onclick={() => void setPin()} disabled={savingPin}>Aggiorna PIN</button>
      </div>
    </section>

    {#if error}<p class="import-error">{error}</p>{/if}
    {#if notice}<p class="hint">{notice}</p>{/if}
  </div>
</div>
