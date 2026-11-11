<script lang="ts">
  // Scelta dei pacchetti al primo avvio.
  //
  // L'installer è snello: i repertori grandi (libreria materiali, modello
  // neurale, voci) non sono inclusi. Qui l'utente vede cosa manca e sceglie
  // cosa vuole configurare. Nessun download parte da questa finestra: le
  // scelte vengono passate al Gestore pacchetti, dove ogni operazione è
  // esplicita. «Più tardi» lascia la finestra per il prossimo avvio.

  interface Props {
    onOpenPackages: (requested: string[]) => void;
    /// `remember` = false quando l'utente sceglie «Più tardi»: in quel caso la
    /// finestra ricompare al prossimo avvio.
    onClose: (remember: boolean) => void;
  }

  let { onOpenPackages, onClose }: Props = $props();

  type OptionalPackage = {
    id: string;
    name: string;
    note: string;
    size: string;
    /// Il modello neurale non viene mai scaricato senza una scelta esplicita.
    offByDefault?: boolean;
  };

  const optionalPackages: OptionalPackage[] = [
    {
      id: "library",
      name: "Libreria materiali",
      note: "Modelli, tabelle e storie pronte da aprire. Senza libreria l'app funziona: la Libreria resta vuota.",
      size: "circa 530 MB sul disco",
    },
    {
      id: "arasaac",
      name: "Pittogrammi ARASAAC",
      note: "Le immagini delle parole del lessico, scaricate da ARASAAC e conservate nella cartella dati dell'utente.",
      size: "a richiesta, pochi MB per volta",
    },
    {
      id: "global-symbols",
      name: "Global Symbols",
      note: "Vocabolario di base in inglese con licenza e attribuzione, tradotto con il glossario locale.",
      size: "a richiesta",
    },
    {
      id: "ai-model",
      name: "Modello neurale locale",
      note: "Suggerimenti contestuali offline. Richiede circa 4 GB di RAM libera; si può installare in un secondo momento.",
      size: "circa 470 MB",
      offByDefault: true,
    },
    {
      id: "voice",
      name: "Voce neurale italiana",
      note: "Lettura offline con Piper. In assenza della voce, la sintesi usa la voce di sistema.",
      size: "circa 60 MB per voce",
    },
  ];

  let selected = $state<string[]>([]);

  function toggle(id: string) {
    selected = selected.includes(id)
      ? selected.filter((entry) => entry !== id)
      : [...selected, id];
  }
</script>

<div class="firstrun" role="dialog" aria-modal="true" aria-labelledby="firstrun-title">
  <div class="firstrun__card">
    <header class="firstrun__head">
      <div>
        <h1 id="firstrun-title">Cosa vuoi installare?</h1>
        <p>
          Il programma funziona già offline: questa installazione contiene i repertori di base.
          Le raccolte grandi sono opzionali e si aggiungono solo se le scegli.
        </p>
      </div>
      <button class="firstrun__close" onclick={() => onClose(true)} aria-label="Chiudi">×</button>
    </header>

    <fieldset class="firstrun__list">
      <legend class="sr-only">Pacchetti opzionali</legend>
      {#each optionalPackages as item (item.id)}
        <label class="firstrun__item" class:firstrun__item--on={selected.includes(item.id)}>
          <input
            type="checkbox"
            checked={selected.includes(item.id)}
            onchange={() => toggle(item.id)}
          />
          <span class="firstrun__text">
            <strong>{item.name}</strong>
            <span>{item.note}</span>
            <span class="firstrun__size">{item.size}{item.offByDefault ? " · non selezionato per impostazione predefinita" : ""}</span>
          </span>
        </label>
      {/each}
    </fieldset>

    <p class="firstrun__hint">
      Nessun download parte automaticamente. Le scelte aprono il Gestore pacchetti, dove puoi
      installare, rimandare o rinunciare a ogni voce. Tutto resta utilizzabile senza rete tranne i
      download che avvii tu.
    </p>

    <footer class="firstrun__actions">
      <button class="btn" onclick={() => onClose(false)}>Più tardi</button>
      <button class="btn" onclick={() => onClose(true)}>Non mostrare più</button>
      <button class="btn btn--solid" onclick={() => onOpenPackages(selected)}>
        Configura i pacchetti scelti
      </button>
    </footer>
  </div>
</div>

<style>
  .firstrun {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: grid;
    place-items: center;
    padding: 16px;
    background: rgba(15, 23, 42, 0.5);
    overflow: auto;
  }
  .firstrun__card {
    width: min(720px, 100%);
    max-height: min(88vh, 760px);
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
    background: var(--surface, #fff);
    color: var(--ink, #17202a);
    border-radius: 16px;
    box-shadow: 0 24px 60px rgba(15, 23, 42, 0.32);
    overflow: auto;
  }
  .firstrun__head {
    display: flex;
    justify-content: space-between;
    gap: 16px;
  }
  .firstrun__head h1 {
    margin: 0;
    font-size: 1.35rem;
  }
  .firstrun__head p {
    margin: 6px 0 0;
    color: #475569;
    font-size: 0.9rem;
    line-height: 1.45;
  }
  .firstrun__close {
    flex: none;
    width: 40px;
    height: 40px;
    border: 1px solid var(--border, #cfd8e3);
    border-radius: 10px;
    background: transparent;
    font-size: 1.3rem;
    cursor: pointer;
  }
  .firstrun__list {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    border: 0;
  }
  .firstrun__item {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    padding: 12px;
    border: 1px solid var(--border, #cfd8e3);
    border-radius: 12px;
    background: var(--surface, #fff);
    cursor: pointer;
  }
  .firstrun__item--on {
    border-color: var(--accent, #1f5bd8);
    background: var(--accent-soft, #dbe7ff);
  }
  .firstrun__item input {
    margin-top: 3px;
    width: 20px;
    height: 20px;
    flex: none;
  }
  .firstrun__text {
    display: grid;
    gap: 3px;
    font-size: 0.86rem;
    line-height: 1.4;
  }
  .firstrun__size {
    color: #64748b;
    font-size: 0.76rem;
  }
  .firstrun__hint {
    margin: 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: #f1f5f9;
    color: #334155;
    font-size: 0.8rem;
    line-height: 1.45;
  }
  .firstrun__actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 8px;
    position: sticky;
    bottom: 0;
    padding-top: 8px;
    background: var(--surface, #fff);
  }
  .btn {
    border: 1px solid #cbd5e1;
    background: #fff;
    color: #1e293b;
    border-radius: 9px;
    padding: 9px 14px;
    font: inherit;
    font-size: 0.86rem;
    cursor: pointer;
  }
  .btn:hover { background: #f1f5f9; }
  .btn--solid {
    background: #1f5bd8;
    border-color: #1f5bd8;
    color: #fff;
    font-weight: 700;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
