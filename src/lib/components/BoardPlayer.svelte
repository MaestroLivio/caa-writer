<script lang="ts">
  import { api } from "../api";
  import type { BoardCell, BoardSystem, Profile } from "../types";

  interface Props {
    boardId: string;
    profile: Profile;
    onClose: () => void;
  }

  let { boardId, profile, onClose }: Props = $props();

  let board = $state<BoardSystem | null>(null);
  let loading = $state(true);
  let error = $state("");
  let stack = $state<string[]>([]);
  let message = $state<{ label: string; svg?: string | null }[]>([]);
  let mode = $state<"compose" | "touch">(
    (profile.navigation?.mode as "compose" | "touch") ?? "compose"
  );

  const canCompose = $derived(profile.features.includes("compose"));
  const canSpeak = $derived(profile.features.includes("speech"));
  const page = $derived(
    board?.pages.find((item) => item.id === stack[stack.length - 1]) ??
      board?.pages.find((item) => item.id === board?.rootPageId) ??
      null
  );
  const columns = $derived(
    Math.max(
      2,
      Number(profile.accessibility?.columns ?? 4) ||
        (page && page.cells.length <= 6 ? 3 : 4)
    )
  );

  $effect(() => {
    let cancelled = false;
    void (async () => {
      loading = true;
      error = "";
      try {
        const built = await api.boardBuild(boardId);
        if (cancelled) return;
        board = built;
        stack = [built.rootPageId];
        message = [];
      } catch (err) {
        if (!cancelled) error = err instanceof Error ? err.message : String(err);
      } finally {
        if (!cancelled) loading = false;
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  async function speak(text: string) {
    if (!canSpeak || !text.trim()) return;
    try {
      await api.speak(text, null, 1, undefined);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function navigate(target: string) {
    if (!board) return;
    if (!board.pages.some((item) => item.id === target)) {
      error = "Collegamento non valido: pagina mancante.";
      return;
    }
    stack = [...stack, target];
  }

  function goBack() {
    stack = stack.slice(0, -1);
  }

  function goHome() {
    if (board) stack = [board.rootPageId];
  }

  function selectCell(cell: BoardCell) {
    const action = cell.action?.type ?? (mode === "compose" ? "append" : "speak");
    if (action === "navigate") {
      const target = cell.action?.targetPageId ?? "";
      if (cell.spokenText) void speak(cell.spokenText);
      navigate(target);
      return;
    }
    if (action === "back") {
      goBack();
      return;
    }
    if (action === "home") {
      goHome();
      return;
    }
    if (action === "append") {
      if (!canCompose) {
        void speak(cell.spokenText || cell.label);
        return;
      }
      message = [...message, { label: cell.spokenText || cell.label, svg: cell.svg }];
      if (mode === "touch") void speak(cell.spokenText || cell.label);
      return;
    }
    if (action === "speak") {
      void speak(cell.spokenText || cell.label);
      return;
    }
    if (mode === "compose" && canCompose) {
      message = [...message, { label: cell.spokenText || cell.label, svg: cell.svg }];
    } else {
      void speak(cell.spokenText || cell.label);
    }
  }

  function speakMessage() {
    const text = message.map((item) => item.label).join(" ").trim();
    if (!text) return;
    void speak(text);
  }

  function removeLast() {
    message = message.slice(0, -1);
  }

  function clearMessage() {
    message = [];
  }

  const messageText = $derived(message.map((item) => item.label).join(" "));
</script>

<div class="board-player" data-theme={profile.age_theme}>
  <header class="board-player__bar">
    <button class="board-btn" onclick={onClose} aria-label="Chiudi">×</button>
    <button class="board-btn" onclick={goHome} disabled={!board} aria-label="Home">⌂ Home</button>
    <button class="board-btn" onclick={goBack} disabled={stack.length <= 1} aria-label="Indietro">‹ Indietro</button>
    <strong class="board-player__title">{page?.title ?? board?.title ?? "Comunicazione"}</strong>
    {#if canCompose}
      <div class="board-player__mode" role="group" aria-label="Modalità comunicazione">
        <button class:active={mode === "compose"} onclick={() => (mode = "compose")}>Componi frase</button>
        <button class:active={mode === "touch"} onclick={() => (mode = "touch")}>Interagisci</button>
      </div>
    {/if}
  </header>

  {#if loading}
    <p class="board-player__status">Caricamento del sistema comunicativo…</p>
  {:else if error && !board}
    <p class="board-player__status board-player__status--error">{error}</p>
  {:else if page}
    <main class="board-grid" style={`grid-template-columns: repeat(${columns}, minmax(0, 1fr));`}>
      {#each page.cells as cell (cell.id)}
        <button
          class="board-cell"
          class:board-cell--link={cell.action?.type === "navigate"}
          class:board-cell--nav={cell.action?.type === "back" || cell.action?.type === "home"}
          onclick={() => selectCell(cell)}
          title={cell.spokenText || cell.label}
        >
          <span class="board-cell__symbol">
            {#if cell.svg}
              {@html cell.svg}
            {:else}
              <span class="board-cell__empty">?</span>
            {/if}
            {#if cell.approximate}<span class="board-cell__approx" title="Pittogramma approssimativo">≈</span>{/if}
          </span>
          <span class="board-cell__label">{cell.label}</span>
          {#if cell.action?.type === "navigate"}<span class="board-cell__chevron" aria-hidden="true">›</span>{/if}
        </button>
      {/each}
    </main>
  {/if}

  {#if canCompose && message.length > 0}
    <footer class="board-message">
      <div class="board-message__chips">
        {#each message as item, index (index)}
          <span class="board-message__chip">{item.label}</span>
        {/each}
      </div>
      <div class="board-message__text">{messageText}</div>
      <div class="board-message__actions">
        {#if canSpeak}<button class="board-btn board-btn--primary" onclick={speakMessage}>Ascolta frase</button>{/if}
        <button class="board-btn" onclick={removeLast}>Cancella ultimo</button>
        <button class="board-btn" onclick={clearMessage}>Svuota</button>
      </div>
    </footer>
  {/if}

  {#if error && board}
    <p class="board-player__error">{error}</p>
  {/if}
</div>
