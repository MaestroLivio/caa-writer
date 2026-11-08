<script lang="ts">
  import { api } from "../api";
  import type { OpenedUniversalDocument } from "../types";

  interface Props {
    onClose: () => void;
    onCreated: (opened: OpenedUniversalDocument) => void;
    onToast: (message: string, kind?: "info" | "warn" | "error") => void;
  }

  let { onClose, onCreated, onToast }: Props = $props();

  let title = $state("La mia storia");
  let kind = $state("social");
  let layout = $state("image_top");
  let pages = $state(10);
  let setting = $state("scuola");
  let characters = $state("bambino, maestra");
  let subject = $state("una situazione da capire insieme");
  let busy = $state(false);
  let error = $state("");

  async function generate() {
    if (busy) return;
    if (!title.trim()) {
      error = "Il titolo è obbligatorio.";
      return;
    }
    busy = true;
    error = "";
    try {
      const opened = await api.storyCreate({
        id: `story-${Date.now().toString(36)}`,
        title: title.trim(),
        kind,
        layout,
        pages,
        setting: setting.trim() || "scuola",
        characters: characters.split(",").map((value) => value.trim()).filter(Boolean),
        subject: subject.trim() || title.trim(),
      });
      onCreated(opened);
      onToast("Storia creata nell'editor universale.", "info");
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }
</script>

<div class="story-backdrop" role="presentation" onclick={(event) => { if (event.target === event.currentTarget) onClose(); }}>
  <div class="story-dialog" role="dialog" aria-modal="true" aria-labelledby="story-title">
    <header class="story-head">
      <div>
        <h2 id="story-title">Crea una storia illustrata in CAA</h2>
        <p>
          La storia viene costruita nell'editor universale: scene vettoriali con pittogrammi
          indipendenti, testo CAA, collegamenti e pagine modificabili.
        </p>
      </div>
      <button class="btn" onclick={onClose} aria-label="Chiudi">×</button>
    </header>

    <div class="story-grid">
      <label class="field field--full"><span>Titolo</span><input bind:value={title} /></label>
      <label class="field"><span>Tipo</span>
        <select bind:value={kind}>
          <option value="social">Storia sociale</option>
          <option value="routine">Routine</option>
          <option value="emotion">Emozioni</option>
          <option value="educational">Storia didattica</option>
          <option value="narrative">Racconto</option>
        </select>
      </label>
      <label class="field"><span>Impaginazione</span>
        <select bind:value={layout}>
          <option value="image_top">Immagine sopra, testo sotto</option>
          <option value="pictograms_top">Pittogrammi sopra, immagine sotto</option>
          <option value="image_left">Immagine a sinistra</option>
          <option value="image_right">Immagine a destra</option>
        </select>
      </label>
      <label class="field"><span>Pagine (10–20)</span><input type="number" min="10" max="20" bind:value={pages} /></label>
      <label class="field"><span>Ambientazione</span><input bind:value={setting} placeholder="scuola, casa, parco…" /></label>
      <label class="field field--full"><span>Personaggi (separati da virgola)</span><input bind:value={characters} placeholder="bambino, maestra" /></label>
      <label class="field field--full"><span>Argomento / obiettivo</span><textarea rows="2" bind:value={subject}></textarea></label>
    </div>

    <p class="story-note">
      Questa versione usa il <strong>generatore vettoriale offline</strong>: ogni personaggio e oggetto
      è un pittogramma modificabile. Non viene usato un modello diffusion finché un runtime e un
      modello non sono stati verificati su questo computer.
    </p>
    {#if error}<p class="story-error" role="alert">{error}</p>{/if}
    <footer class="story-actions">
      <button class="btn" onclick={onClose} disabled={busy}>Annulla</button>
      <button class="btn btn--solid" onclick={() => void generate()} disabled={busy}>
        {busy ? "Creo la storia…" : "Genera nell'editor"}
      </button>
    </footer>
  </div>
</div>

<style>
  .story-backdrop { position: fixed; inset: 0; z-index: 2500; background: #0f172a99; display: flex; align-items: center; justify-content: center; padding: 20px; overflow: auto; }
  .story-dialog { width: min(700px, 100%); background: #fff; color: #0f172a; border-radius: 18px; padding: 22px; box-shadow: 0 24px 70px #0006; }
  .story-head { display: flex; gap: 12px; align-items: flex-start; justify-content: space-between; }
  .story-head h2 { margin: 0; font-size: 21px; }
  .story-head p { margin: 6px 0 0; color: #64748b; font-size: 12px; max-width: 560px; }
  .story-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin: 18px 0; }
  .field { display: grid; gap: 4px; font-size: 12px; color: #475569; }
  .field--full { grid-column: 1 / -1; }
  .field input, .field select, .field textarea { width: 100%; box-sizing: border-box; border: 1px solid #cbd5e1; border-radius: 8px; padding: 8px 10px; font: inherit; color: #0f172a; background: #f8fafc; }
  .story-note { margin: 0 0 12px; padding: 10px 12px; border-radius: 10px; background: #ecfdf5; color: #115e59; font-size: 12px; }
  .story-error { margin: 0 0 12px; color: #b91c1c; font-size: 12px; font-weight: 600; }
  .story-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .btn { border: 1px solid #cbd5e1; background: #fff; color: #1e293b; border-radius: 9px; padding: 8px 13px; font: inherit; font-size: 12px; font-weight: 600; cursor: pointer; }
  .btn--solid { background: #0f766e; border-color: #0f766e; color: #fff; }
  .btn:disabled { opacity: 0.55; cursor: wait; }
  @media (max-width: 560px) { .story-grid { grid-template-columns: 1fr; } .field--full { grid-column: auto; } }
</style>
