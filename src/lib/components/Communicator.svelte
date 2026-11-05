<script lang="ts">
  import type { LayoutDocument, PlacedElement } from "../types";
  import { language, strings } from "../i18n";

  let {
    doc,
    onSpeak,
    onClose,
  }: {
    doc: LayoutDocument;
    onSpeak: (text: string) => void;
    onClose: () => void;
  } = $props();

  type MessageUnit = { key: string; label: string };

  let message = $state<MessageUnit[]>([]);
  let pageIndex = $state(0);
  let history = $state<number[]>([]);
  let unitSeq = 0;

  const copy = $derived(strings[$language]);
  const currentPage = $derived(doc.pages[pageIndex] ?? doc.pages[0]);
  const cells = $derived((currentPage?.elements ?? []).filter((cell) => !cell.hidden));

  function goTo(index: number) {
    if (index === pageIndex || index < 0 || index >= doc.pages.length) return;
    history = [...history, pageIndex];
    pageIndex = index;
  }

  function back() {
    const previous = history[history.length - 1];
    if (previous === undefined) return;
    history = history.slice(0, -1);
    pageIndex = previous;
  }

  function home() {
    if (pageIndex === 0) return;
    history = [...history, pageIndex];
    pageIndex = 0;
  }

  function addCell(cell: PlacedElement) {
    unitSeq += 1;
    message = [...message, { key: `${cell.id}-${unitSeq}`, label: cell.label || "?" }];
  }

  function removeLast() {
    message = message.slice(0, -1);
  }

  function clearMessage() {
    message = [];
  }

  function speakMessage() {
    if (message.length === 0) return;
    const text = message
      .map((unit) => unit.label)
      .join(" ")
      .replace(/\s+([,.;:!?])/g, "$1")
      .trim();
    onSpeak(/[.!?]$/.test(text) ? text : `${text}.`);
  }

  function handleKey(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
    }
    if (event.key === "Backspace" && message.length > 0) {
      event.preventDefault();
      removeLast();
    }
  }
</script>

<svelte:window on:keydown={handleKey} />

<section
  class="communicator"
  aria-label={copy.communicator.dialogLabel(doc.spec.title || copy.communicator.titleFallback)}
>
  <header class="communicator__bar">
    <button type="button" class="btn" onclick={home} disabled={pageIndex === 0}
      >{copy.communicator.home}</button
    >
    <button type="button" class="btn" onclick={back} disabled={history.length === 0}
      >{copy.communicator.back}</button
    >
    <h2>{doc.spec.title || copy.communicator.titleFallback}</h2>
    <span class="hint">{copy.communicator.page(pageIndex + 1, doc.pages.length)}</span>
    <span class="grow"></span>
    <button type="button" class="btn" onclick={onClose}>{copy.communicator.exit}</button>
  </header>

  <div class="communicator__grid" role="group" aria-label={copy.communicator.gridLabel}>
    {#each cells as cell (cell.id)}
      <button
        type="button"
        class="cell"
        onclick={() => addCell(cell)}
        aria-label={copy.communicator.add(cell.label || copy.communicator.unnamedSymbol)}
      >
        {#if cell.svg}
          <span class="cell__symbol">{@html cell.svg}</span>
        {/if}
        <span class="cell__label">{cell.label}</span>
      </button>
    {/each}
    {#if cells.length === 0}
      <p class="hint">{copy.communicator.emptyPage}</p>
    {/if}
  </div>

  <footer class="communicator__message">
    <div class="message" aria-live="polite" aria-label={copy.communicator.messageLabel}>
      {#if message.length === 0}
        <span class="hint">{copy.communicator.emptyMessage}</span>
      {:else}
        {#each message as unit (unit.key)}
          <span class="chip">{unit.label}</span>
        {/each}
      {/if}
    </div>
    <div class="communicator__actions">
      <button type="button" class="btn" onclick={removeLast} disabled={message.length === 0}>
        {copy.communicator.removeLast}
      </button>
      <button type="button" class="btn" onclick={clearMessage} disabled={message.length === 0}>
        {copy.communicator.clear}
      </button>
      <button
        type="button"
        class="btn btn--solid"
        onclick={speakMessage}
        disabled={message.length === 0}
      >
        {copy.communicator.speak}
      </button>
    </div>
  </footer>
</section>

<style>
  .communicator {
    position: fixed;
    inset: 0;
    z-index: 80;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    background: var(--surface, #f7f7f8);
    color: var(--text, #16161a);
  }

  .communicator__bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .communicator__bar h2 {
    margin: 0 6px;
    font-size: 1.1rem;
  }

  .grow {
    flex: 1 1 auto;
  }

  .communicator__grid {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
    gap: 10px;
    align-content: start;
  }

  .cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 132px;
    padding: 10px;
    border: 2px solid var(--border, #b9b9c0);
    border-radius: 12px;
    background: #ffffff;
    color: #141414;
    cursor: pointer;
    font: inherit;
  }

  .cell:hover {
    border-color: var(--accent, #2463eb);
  }

  .cell:focus-visible {
    outline: 4px solid var(--accent, #2463eb);
    outline-offset: 2px;
  }

  .cell__symbol {
    width: 72px;
    height: 72px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .cell__symbol :global(svg) {
    width: 100%;
    height: 100%;
  }

  .cell__label {
    font-size: 1rem;
    font-weight: 600;
    text-align: center;
  }

  .communicator__message {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    padding: 10px;
    border-top: 2px solid var(--border, #d3d3da);
    background: var(--surface, #fff);
  }

  .message {
    flex: 1 1 320px;
    min-height: 52px;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    padding: 8px;
    border: 1px dashed var(--border, #b9b9c0);
    border-radius: 10px;
  }

  .chip {
    padding: 4px 10px;
    border-radius: 999px;
    background: #e8eefc;
    color: #12234a;
    font-weight: 600;
  }

  .communicator__actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  @media (max-width: 640px) {
    .communicator__grid {
      grid-template-columns: repeat(auto-fit, minmax(96px, 1fr));
    }
  }
</style>
