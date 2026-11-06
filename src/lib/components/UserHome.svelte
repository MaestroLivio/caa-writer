<script lang="ts">
  import type { Profile, ProfileBoard } from "../types";

  interface Props {
    profile: Profile;
    boards: ProfileBoard[];
    onOpenBoard: (boardId: string) => void;
    onOpenEditor: () => void;
    onOpenCreator: () => void;
  }

  let { profile, boards, onOpenBoard, onOpenEditor, onOpenCreator }: Props = $props();

  const visibleBoards = $derived(boards.filter((board) => board.enabled));
  const canEdit = $derived(profile.features.includes("editor"));
  const canSearch = $derived(profile.features.includes("search"));
</script>

<div class="user-home" data-theme={profile.age_theme}>
  <header class="user-home__head">
    <div class="user-home__identity">
      <span class="user-home__avatar" aria-hidden="true">{profile.avatar || profile.name.slice(0, 1)}</span>
      <div>
        <h1>{profile.name}</h1>
        <p>Le mie comunicazioni</p>
      </div>
    </div>
    <button class="user-home__creator" onclick={onOpenCreator}>Area Creator</button>
  </header>

  {#if visibleBoards.length > 0}
    <section class="user-home__grid" aria-label="Tabelle e materiali">
      {#each visibleBoards as board (board.boardId)}
        <button class="user-card" onclick={() => onOpenBoard(board.boardId)}>
          <span class="user-card__icon" aria-hidden="true">▦</span>
          <span class="user-card__title">{board.title}</span>
          <span class="user-card__meta">{board.pages} pagine</span>
        </button>
      {/each}
      {#if canEdit}
        <button class="user-card" onclick={onOpenEditor}>
          <span class="user-card__icon" aria-hidden="true">✎</span>
          <span class="user-card__title">Scrivo</span>
          <span class="user-card__meta">Editor semplificato</span>
        </button>
      {/if}
    </section>
  {:else}
    <section class="user-home__empty">
      <h2>Nessun materiale assegnato</h2>
      <p>Chiedi al Creator di assegnare una tabella o un materiale a questo profilo.</p>
      {#if canEdit}
        <button class="btn btn--solid" onclick={onOpenEditor}>Apri l'editor</button>
      {/if}
    </section>
  {/if}

  {#if canSearch}
    <p class="user-home__hint">Puoi aprire una tabella e usare il pulsante Home per tornare qui.</p>
  {/if}
</div>
