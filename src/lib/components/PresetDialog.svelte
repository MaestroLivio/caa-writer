<script lang="ts">
  interface Props {
    onSave: (name: string) => void;
    onClose: () => void;
  }

  let { onSave, onClose }: Props = $props();
  let name = $state("");
  let input: HTMLInputElement | undefined = $state();

  $effect(() => {
    input?.focus();
  });
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onClose()} />

<div class="dialog-backdrop">
  <div class="report-dialog" role="dialog" aria-modal="true" aria-label="Salva preset dell'area di lavoro">
    <header class="report-dialog__head">
      <h2>Salva preset dell'area di lavoro</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
    </header>
    <label for="preset-name">Nome del preset</label>
    <input
      id="preset-name"
      bind:this={input}
      bind:value={name}
      placeholder="Es. Scrittura serale"
      onkeydown={(event) => {
        if (event.key === "Enter" && name.trim()) {
          onSave(name.trim());
          onClose();
        }
      }}
    />
    <div style="display:flex;gap:8px;justify-content:flex-end;margin-top:12px">
      <button class="btn btn--ghost" onclick={onClose}>Annulla</button>
      <button
        class="btn btn--solid"
        disabled={!name.trim()}
        onclick={() => {
          onSave(name.trim());
          onClose();
        }}>Salva</button
      >
    </div>
  </div>
</div>
