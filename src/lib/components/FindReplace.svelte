<script lang="ts">
  interface Props {
    onFind: (query: string, forward: boolean) => void;
    onReplace: (query: string, replacement: string) => void;
    onReplaceAll: (query: string, replacement: string) => void;
    countMatches: (query: string) => number;
    onClose: () => void;
  }

  let { onFind, onReplace, onReplaceAll, countMatches, onClose }: Props = $props();
  let query = $state("");
  let replacement = $state("");
  let input: HTMLInputElement | undefined = $state();
  const matches = $derived(query.trim() ? countMatches(query) : 0);

  $effect(() => {
    input?.focus();
    input?.select();
  });

  function close() {
    onClose();
  }
</script>

<div class="findbar" role="dialog" aria-label="Trova e sostituisci">
  <div class="findbar__row">
    <label for="find-query">Trova</label>
    <input
      id="find-query"
      bind:this={input}
      bind:value={query}
      onkeydown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          onFind(query, !event.shiftKey);
        } else if (event.key === "Escape") {
          close();
        }
      }}
    />
    <span class="hint">{matches} {matches === 1 ? "occorrenza" : "occorrenze"}</span>
    <button class="btn btn--ghost" onclick={() => onFind(query, false)} disabled={!query.trim()}>Precedente</button>
    <button class="btn" onclick={() => onFind(query, true)} disabled={!query.trim()}>Successivo</button>
    <button class="btn btn--ghost" onclick={close} aria-label="Chiudi trova e sostituisci">×</button>
  </div>
  <div class="findbar__row">
    <label for="find-replace">Sostituisci</label>
    <input
      id="find-replace"
      bind:value={replacement}
      onkeydown={(event) => {
        if (event.key === "Escape") close();
      }}
    />
    <button class="btn" onclick={() => onReplace(query, replacement)} disabled={!query.trim()}>
      Sostituisci
    </button>
    <button class="btn btn--ghost" onclick={() => onReplaceAll(query, replacement)} disabled={!query.trim()}>
      Sostituisci tutto
    </button>
  </div>
</div>
