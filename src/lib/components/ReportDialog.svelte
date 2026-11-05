<script lang="ts">
  interface Props {
    title: string;
    lines: string[];
    onClose: () => void;
  }

  let { title, lines, onClose }: Props = $props();
</script>

<svelte:window onkeydown={(event) => event.key === "Escape" && onClose()} />

<div class="dialog-backdrop">
  <div class="report-dialog" role="dialog" aria-modal="true" aria-label={title}>
    <header class="report-dialog__head">
      <h2>{title}</h2>
      <button class="btn btn--ghost" onclick={onClose} aria-label="Chiudi">×</button>
    </header>
    <div class="report-dialog__body">
      {#each lines as line}
        <p>{line}</p>
      {/each}
    </div>
  </div>
</div>
