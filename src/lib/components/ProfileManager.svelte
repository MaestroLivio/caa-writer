<script lang="ts">
  import { language, strings } from "../i18n";
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

  // Modal di conferma per eliminazione profilo
  let confirmDeleteOpen = $state(false);
  let confirmDeleteProfileId = $state("");

  function closeHandler(event: KeyboardEvent) {
    if (event.key === "Escape") {
      if (confirmDeleteOpen) {
        confirmDeleteOpen = false;
      } else {
        onClose();
      }
    }
  }

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
    window.addEventListener("keydown", closeHandler);
    return () => {
      window.removeEventListener("keydown", closeHandler);
    };
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
      notice = strings[$language].profileManager.materialRemoved(material.title);
    } catch (err) {
      assigned = { ...assigned, [material.id]: true };
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function addAssigned(material: LibraryEntry) {
    assigned = { ...assigned, [material.id]: true };
    notice = strings[$language].profileManager.materialAdded(material.title);
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
      notice = strings[$language].profileManager.profileSaved;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function confirmRemoveProfile() {
    if (!draft || !profiles.some((profile) => profile.id === draft!.id)) return;
    confirmDeleteProfileId = draft.id;
    confirmDeleteOpen = true;
  }

  async function executeRemoveProfile() {
    const id = confirmDeleteProfileId;
    confirmDeleteOpen = false;
    if (!id) return;
    busy = true;
    try {
      await api.profileDelete(id, creatorPin || null);
      await onChanged();
      const next = profiles.find((profile) => profile.id !== id);
      if (next) await select(next.id);
      else newProfile();
      notice = strings[$language].profileManager.profileDeleted;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function setPin() {
    if (savingPin) return;
    if (pin && pin.length < 4) {
      error = strings[$language].profileManager.pinTooShort;
      return;
    }
    if (pin !== pinConfirm) {
      error = strings[$language].profileManager.pinsMismatch;
      return;
    }
    savingPin = true;
    error = "";
    try {
      await api.creatorSetPin(pin.trim() ? pin.trim() : null, creatorPin || null);
      notice = pin.trim() ? strings[$language].profileManager.pinUpdated : strings[$language].profileManager.pinRemoved;
      pin = "";
      pinConfirm = "";
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      savingPin = false;
    }
  }

  const pm = $derived.by(() => strings[$language].profileManager);
</script>

<div class="dialog-backdrop">
  <div class="report-dialog profile-manager" role="dialog" aria-modal="true" aria-label={pm.title}>
    <header class="report-dialog__head">
      <h2>{pm.title}</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label={pm.close}>×</button>
    </header>

    <div class="profile-manager__body">
      <aside class="profile-list" aria-label={pm.profileListLabel}>
        {#each profiles as profile (profile.id)}
          <button
            class="profile-list__item"
            class:active={selectedId === profile.id}
            onclick={() => void select(profile.id)}
          >{profile.name}</button>
        {/each}
        <button class="btn btn--ghost" onclick={newProfile}>{pm.newProfile}</button>
      </aside>

      {#if draft}
        <section class="profile-editor">
          <div class="profile-row">
            <label class="profile-field">
              {pm.name}
              <input bind:value={draft.name} />
            </label>
            <label class="profile-field">
              {pm.theme}
              <select bind:value={draft.age_theme}>
                <option value="infanzia">{pm.themes.infanzia}</option>
                <option value="primaria">{pm.themes.primaria}</option>
                <option value="secondaria">{pm.themes.secondaria}</option>
                <option value="adulto">{pm.themes.adulto}</option>
                <option value="contrasto">{pm.themes.contrasto}</option>
                <option value="minimal">{pm.themes.minimal}</option>
              </select>
            </label>
            <label class="profile-field">
              {pm.editorLevel}
              <select bind:value={draft.editor_level}>
                <option value={1}>{pm.editorLevels[1]}</option>
                <option value={2}>{pm.editorLevels[2]}</option>
                <option value={3}>{pm.editorLevels[3]}</option>
              </select>
            </label>
          </div>

          <fieldset class="profile-features">
            <legend>{pm.enabledFeatures}</legend>
            {#each allFeatures as feature (feature)}
              <label>
                <input
                  type="checkbox"
                  checked={draft.features.includes(feature)}
                  onchange={() => toggleFeature(feature)}
                />
                {pm.features[feature as keyof typeof pm.features] ?? feature}
              </label>
            {/each}
          </fieldset>

          <fieldset class="profile-boards">
            <legend>{pm.assignedMaterials(String(assignedCount))}</legend>
            <p class="hint">
              {pm.assignedHint}
            </p>
            {#if assignedMaterials.length === 0}
              <p class="hint">{pm.noAssignedMaterials}</p>
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
                    >{pm.removeFromProfile}</button>
                  </div>
                {/each}
              </div>
            {/if}
            <button class="btn btn--ghost" onclick={() => (addOpen = !addOpen)}>
              {addOpen ? pm.closeMaterialsList : pm.addMaterials}
            </button>
            {#if addOpen}
              <input
                class="profile-search"
                bind:value={materialQuery}
                placeholder={pm.searchLibrary}
                aria-label={pm.searchLibrary}
              />
              <div class="profile-materials">
                {#each addableMaterials as material (material.id)}
                  <div class="profile-material">
                    <span>{material.title}</span>
                    <span class="hint">{material.category || material.kind} · {material.pages} pagine</span>
                    <button class="btn" onclick={() => addAssigned(material)}>{pm.addToProfile}</button>
                  </div>
                {/each}
                {#if addableMaterials.length === 0}
                  <p class="hint">{pm.allAssigned}</p>
                {/if}
              </div>
            {/if}
          </fieldset>

          <div class="profile-actions">
            <button class="btn btn--solid" onclick={() => void save()} disabled={busy}>{pm.saveProfile}</button>
            <button class="btn btn--ghost" onclick={confirmRemoveProfile} disabled={busy || !selectedId}>{pm.deleteProfile}</button>
            <button
              class="btn btn--ghost"
              onclick={() => void api.profileSetActive(draft!.id).then(onChanged)}
              disabled={busy || !selectedId}
            >{pm.setAsDefault}</button>
          </div>
        </section>
      {/if}
    </div>

    <section class="profile-pin">
      <h3>{pm.creatorPinTitle}</h3>
      <p class="hint">{pm.creatorPinHint}</p>
      <div class="profile-row">
        <label class="profile-field">
          {pm.newPin}
          <input type="password" bind:value={pin} />
        </label>
        <label class="profile-field">
          {pm.confirmPin}
          <input type="password" bind:value={pinConfirm} />
        </label>
        <button class="btn" onclick={() => void setPin()} disabled={savingPin}>{pm.updatePin}</button>
      </div>
    </section>

    {#if error}<p class="import-error">{pm.errorPrefix}{error}</p>{/if}
    {#if notice}<p class="hint">{notice}</p>{/if}

    {#if confirmDeleteOpen}
      <div class="confirm-modal" role="alertdialog" aria-modal="true" aria-labelledby="confirm-title">
        <div class="confirm-modal__card">
          <h3 id="confirm-title">{pm.confirmDelete(draft?.name ?? "")}</h3>
          <div class="confirm-modal__actions">
            <button class="btn" onclick={() => (confirmDeleteOpen = false)}>{pm.cancel}</button>
            <button class="btn btn--solid" onclick={executeRemoveProfile}>{pm.delete}</button>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .confirm-modal {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    background: rgba(15, 23, 42, 0.5);
  }
  .confirm-modal__card {
    background: #fff;
    border-radius: 12px;
    padding: 20px 24px;
    min-width: 320px;
    max-width: 90vw;
    box-shadow: 0 24px 60px rgba(15, 23, 42, 0.3);
  }
  .confirm-modal__card h3 {
    margin: 0 0 16px;
    font-size: 1rem;
    color: #0f172a;
  }
  .confirm-modal__actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
