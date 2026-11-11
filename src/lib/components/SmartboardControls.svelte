<script lang="ts">
  interface Props {
    onInsert: (text: string) => void;
    onBackspace: () => void;
    onEnter: () => void;
    onUndo: () => void;
    onRedo: () => void;
    onSpeak: () => void;
  }

  let { onInsert, onBackspace, onEnter, onUndo, onRedo, onSpeak }: Props = $props();

  let open = $state(false);
  let keyboard = $state(false);
  let numbers = $state(false);
  let shift = $state(false);

  const letterRows = [
    ["q", "w", "e", "r", "t", "y", "u", "i", "o", "p"],
    ["a", "s", "d", "f", "g", "h", "j", "k", "l"],
    ["z", "x", "c", "v", "b", "n", "m"],
  ];
  const numberRows = [
    ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"],
    [".", ",", "?", "!", ":", ";", "'", "\"", "-", "+"],
  ];
  const accents = ["à", "è", "é", "ì", "ò", "ù"];

  function insert(value: string) {
    onInsert(value);
  }

  function letter(value: string) {
    insert(shift ? value.toUpperCase() : value);
    shift = false;
  }

  function space() {
    onInsert(" ");
  }
</script>

<div class="smartboard" aria-label="Controlli per smartboard">
  {#if !open}
    <button class="smartboard__fab" onclick={() => (open = true)} title="Aggiungi controlli smartboard">
      <span aria-hidden="true">⌨</span>
      <span>Controlli</span>
    </button>
  {:else}
    <div class="smartboard__panel">
      <div class="smartboard__bar">
        <button
          class="sb-btn"
          class:sb-btn--active={keyboard}
          aria-pressed={keyboard}
          onclick={() => (keyboard = !keyboard)}
          title="Mostra o nascondi la tastiera a schermo"
        >Tastiera</button>
        <button class="sb-btn" onclick={space} title="Inserisci uno spazio">Spazio</button>
        <button class="sb-btn" onclick={onBackspace} title="Cancella il carattere precedente">⌫</button>
        <button class="sb-btn" onclick={onEnter} title="Nuovo paragrafo">Invio</button>
        <button class="sb-btn" onclick={onUndo} title="Annulla">↶</button>
        <button class="sb-btn" onclick={onRedo} title="Ripeti">↷</button>
        <button class="sb-btn" onclick={onSpeak} title="Ascolta la frase corrente">Ascolta</button>
        <button class="sb-btn sb-btn--close" onclick={() => { open = false; keyboard = false; }} title="Rimuovi i controlli">×</button>
      </div>

      {#if keyboard}
        <div class="smartboard__keyboard" aria-label="Tastiera a schermo">
          <div class="sb-row">
            <button class="sb-key sb-key--wide" onclick={() => (numbers = !numbers)}>
              {numbers ? "ABC" : "123"}
            </button>
            {#if numbers}
              {#each numberRows[0] as key (key)}
                <button class="sb-key" onclick={() => insert(key)}>{key}</button>
              {/each}
            {:else}
              {#each letterRows[0] as key (key)}
                <button class="sb-key" onclick={() => letter(key)}>{shift ? key.toUpperCase() : key}</button>
              {/each}
            {/if}
            <button class="sb-key sb-key--wide" onclick={onBackspace} aria-label="Cancella">⌫</button>
          </div>

          <div class="sb-row">
            {#if numbers}
              {#each numberRows[1] as key (key)}
                <button class="sb-key" onclick={() => insert(key)}>{key}</button>
              {/each}
              <button class="sb-key sb-key--wide" onclick={() => (numbers = false)}>ABC</button>
            {:else}
              {#each letterRows[1] as key (key)}
                <button class="sb-key" onclick={() => letter(key)}>{shift ? key.toUpperCase() : key}</button>
              {/each}
            {/if}
          </div>

          <div class="sb-row">
            {#if numbers}
              <button class="sb-key sb-key--wide" onclick={() => (numbers = false)}>ABC</button>
              {#each accents as key (key)}
                <button class="sb-key" onclick={() => insert(key)}>{key}</button>
              {/each}
              <button class="sb-key sb-key--wide" onclick={() => insert(" ")}>Spazio</button>
            {:else}
              <button
                class="sb-key sb-key--wide"
                class:sb-key--active={shift}
                aria-pressed={shift}
                onclick={() => (shift = !shift)}
                title="Maiuscole"
              >⇧</button>
              {#each letterRows[2] as key (key)}
                <button class="sb-key" onclick={() => letter(key)}>{shift ? key.toUpperCase() : key}</button>
              {/each}
              <button class="sb-key sb-key--wide" onclick={onBackspace} aria-label="Cancella">⌫</button>
            {/if}
          </div>

          <div class="sb-row">
            {#each accents as key (key)}
              <button class="sb-key" onclick={() => insert(key)}>{key}</button>
            {/each}
            <button class="sb-key sb-key--space" onclick={space}>Spazio</button>
            <button class="sb-key sb-key--wide" onclick={onEnter}>Invio</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .smartboard {
    position: fixed;
    left: 12px;
    bottom: 12px;
    z-index: 90;
    max-width: calc(100vw - 24px);
    font-family: var(--font-ui, system-ui, sans-serif);
  }
  .smartboard__fab {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: 1px solid #0f766e;
    background: #0f766e;
    color: #fff;
    border-radius: 999px;
    padding: 7px 10px;
    font: inherit;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    box-shadow: 0 6px 18px #0f172a33;
    touch-action: manipulation;
  }
  .smartboard__fab span[aria-hidden] { font-size: 14px; }
  .smartboard__panel {
    width: min(680px, calc(100vw - 24px));
    background: #ffffff;
    border: 1px solid #cbd5e1;
    border-radius: 14px;
    box-shadow: 0 14px 38px #0f172a40;
    padding: 8px;
    display: grid;
    gap: 8px;
  }
  .smartboard__bar {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    align-items: center;
  }
  .sb-btn {
    border: 1px solid #cbd5e1;
    background: #f8fafc;
    color: #1e293b;
    border-radius: 8px;
    min-height: 34px;
    padding: 4px 9px;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    touch-action: manipulation;
  }
  .sb-btn:hover, .sb-btn--active { background: #ccfbf1; border-color: #0f766e; color: #115e59; }
  .sb-btn--close { margin-left: auto; font-size: 17px; padding: 2px 10px; }
  .smartboard__keyboard { display: grid; gap: 4px; }
  .sb-row { display: flex; gap: 4px; justify-content: center; }
  .sb-key {
    flex: 1 1 0;
    min-width: 0;
    min-height: 36px;
    border: 1px solid #cbd5e1;
    background: #fff;
    color: #0f172a;
    border-radius: 7px;
    font: inherit;
    font-size: 14px;
    font-weight: 600;
    cursor: pointer;
    touch-action: manipulation;
  }
  .sb-key:hover, .sb-key--active { background: #ecfdf5; border-color: #0f766e; }
  .sb-key--wide { flex: 1.5 1 0; font-size: 12px; }
  .sb-key--space { flex: 3 1 0; font-size: 12px; }
  @media (max-width: 620px) {
    .smartboard__panel { padding: 6px; }
    .sb-key { min-height: 32px; font-size: 12px; }
    .sb-btn { min-height: 30px; font-size: 11px; padding: 3px 7px; }
  }
  @media (prefers-reduced-motion: reduce) {
    .smartboard * { scroll-behavior: auto; }
  }
</style>
