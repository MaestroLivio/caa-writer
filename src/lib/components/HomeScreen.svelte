<script lang="ts">
  import type { AppInfo, CreationMode, LibraryStats, TemplateInfo } from "../types";
  import { language, setLanguage, strings, type UiStrings } from "../i18n";
  import {
    clearRecent,
    loadFavorites,
    loadRecent,
    removeRecent,
    toggleFavorite,
    type RecentDoc,
  } from "../recent";

  interface Props {
    stats: LibraryStats | null;
    templates: TemplateInfo[];
    appInfo: AppInfo | null;
    onCreate: (mode: CreationMode) => void;
    onOpenDocument: () => void;
    onOpenRecent: (path: string) => void;
    onOpenLibrary: () => void;
    onOpenSettings: () => void;
    onOpenTemplate: (templateId: string) => void;
  }

  let {
    stats,
    templates,
    appInfo,
    onCreate,
    onOpenDocument,
    onOpenRecent,
    onOpenLibrary,
    onOpenSettings,
    onOpenTemplate,
  }: Props = $props();

  type Card = {
    mode: CreationMode;
    key: keyof UiStrings["cards"];
    icon: string;
  };

  const icon = {
    pen: '<svg viewBox="0 0 48 48" aria-hidden="true"><path d="M8 40l3-9 21-21 6 6-21 21-9 3z" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linejoin="round"/><path d="M28 14l6 6" stroke="currentColor" stroke-width="2.4"/></svg>',
    grid: '<svg viewBox="0 0 48 48" aria-hidden="true"><rect x="7" y="7" width="14" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="2.4"/><rect x="27" y="7" width="14" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="2.4"/><rect x="7" y="27" width="14" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="2.4"/><rect x="27" y="27" width="14" height="14" rx="3" fill="none" stroke="currentColor" stroke-width="2.4"/></svg>',
    calendar:
      '<svg viewBox="0 0 48 48" aria-hidden="true"><rect x="7" y="10" width="34" height="31" rx="4" fill="none" stroke="currentColor" stroke-width="2.4"/><path d="M7 20h34M16 6v8M32 6v8" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/><circle cx="17" cy="27" r="2" fill="currentColor"/><circle cx="25" cy="27" r="2" fill="currentColor"/><circle cx="33" cy="27" r="2" fill="currentColor"/></svg>',
    token: '<svg viewBox="0 0 48 48" aria-hidden="true"><circle cx="17" cy="24" r="10" fill="none" stroke="currentColor" stroke-width="2.4"/><circle cx="34" cy="24" r="8" fill="none" stroke="currentColor" stroke-width="2.4"/><path d="M11 24h3M31 24h3" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/></svg>',
    steps: '<svg viewBox="0 0 48 48" aria-hidden="true"><path d="M9 36h9v-8h9v-8h9V9" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linejoin="round"/><circle cx="9" cy="36" r="3" fill="currentColor"/><circle cx="40" cy="9" r="3" fill="currentColor"/></svg>',
    spark: '<svg viewBox="0 0 48 48" aria-hidden="true"><path d="M24 6l4 12 12 4-12 4-4 12-4-12-12-4 12-4 4-12z" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linejoin="round"/><path d="M37 32l1.6 4.4L43 38l-4.4 1.6L37 44l-1.6-4.4L31 38l4.4-1.6L37 32z" fill="currentColor"/></svg>',
    free: '<svg viewBox="0 0 48 48" aria-hidden="true"><rect x="6" y="9" width="36" height="24" rx="4" fill="none" stroke="currentColor" stroke-width="2.4"/><path d="M15 40h18M24 33v7" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/><path d="M16 25l6-6 5 5 6-8" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/></svg>',
  };

  const cards: Card[] = [
    { mode: "word", key: "word", icon: icon.pen },
    { mode: "grid", key: "grid", icon: icon.grid },
    { mode: "schedule", key: "schedule", icon: icon.calendar },
    { mode: "aba", key: "aba", icon: icon.token },
    { mode: "teacch", key: "teacch", icon: icon.steps },
    { mode: "smart", key: "smart", icon: icon.spark },
    { mode: "free", key: "free", icon: icon.free },
  ];

  // Dizionario reattivo: cambiare lingua ridisegna subito tutta la schermata.
  const copy = $derived(strings[$language]);

  let recent = $state<RecentDoc[]>([]);
  let favorites = $state<string[]>([]);
  let templateQuery = $state("");

  $effect(() => {
    recent = loadRecent();
    favorites = loadFavorites();
  });

  const filteredTemplates = $derived.by(() => {
    const query = templateQuery.trim().toLowerCase();
    const list = query
      ? templates.filter(
          (template) =>
            template.name.toLowerCase().includes(query) ||
            template.category.toLowerCase().includes(query) ||
            template.description.toLowerCase().includes(query)
        )
      : templates;
    return [...list].sort((a, b) => {
      const fa = favorites.includes(a.id) ? 0 : 1;
      const fb = favorites.includes(b.id) ? 0 : 1;
      return fa - fb || a.name.localeCompare(b.name);
    });
  });

  function star(id: string) {
    favorites = toggleFavorite(id);
  }

  function dropRecent(event: MouseEvent, path: string) {
    event.stopPropagation();
    removeRecent(path);
    recent = loadRecent();
  }

  function wipeRecent() {
    clearRecent();
    recent = loadRecent();
  }

  function when(ts: number): string {
    const date = new Date(ts);
    const today = new Date();
    const sameDay =
      date.getDate() === today.getDate() &&
      date.getMonth() === today.getMonth() &&
      date.getFullYear() === today.getFullYear();
    return sameDay
      ? `${copy.today} ${date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`
      : date.toLocaleDateString();
  }
</script>

<div class="home">
  <header class="home__top">
    <div class="home__brand">
      <span class="home__mark" aria-hidden="true">{@html icon.pen}</span>
      <div>
        <h1>CAA Writer</h1>
        <p class="home__tagline">{copy.tagline}</p>
      </div>
    </div>
    <div class="home__actions">
      <div class="lang-switch" role="group" aria-label={copy.languageLabel}>
        <button
          class="btn"
          class:btn--solid={$language === "it"}
          aria-pressed={$language === "it"}
          onclick={() => setLanguage("it")}>Italiano</button
        >
        <button
          class="btn"
          class:btn--solid={$language === "en-GB"}
          aria-pressed={$language === "en-GB"}
          onclick={() => setLanguage("en-GB")}>English (UK)</button
        >
      </div>
      <button class="btn" onclick={onOpenDocument}>{copy.openDocument}</button>
      <button class="btn" onclick={onOpenLibrary}>{copy.library}</button>
      <button class="btn" onclick={onOpenSettings}>{copy.settings}</button>
    </div>
  </header>

  <main class="home__main">
    <section aria-labelledby="create-title">
      <h2 id="create-title" class="home__heading">{copy.createTitle}</h2>
      <div class="cards">
        {#each cards as card (card.mode)}
          <button class="card" class:card--main={card.mode === "word"} onclick={() => onCreate(card.mode)}>
            <span class="card__icon" aria-hidden="true">{@html card.icon}</span>
            <span class="card__title">{copy.cards[card.key].title}</span>
            <span class="card__text">{copy.cards[card.key].description}</span>
            <span class="card__detail">{copy.cards[card.key].detail}</span>
          </button>
        {/each}
      </div>
    </section>

    <section aria-labelledby="recent-title">
      <div class="home__section-head">
        <h2 id="recent-title" class="home__heading">{copy.recentTitle}</h2>
        {#if recent.length > 0}
          <button class="link-btn" onclick={wipeRecent}>{copy.clearRecent}</button>
        {/if}
      </div>
      {#if recent.length === 0}
        <p class="home__empty">{copy.recentEmpty}</p>
      {:else}
        <div class="recent">
          {#each recent as doc (doc.path)}
            <div class="recent__item">
              <button class="recent__open" onclick={() => onOpenRecent(doc.path)}>
                <span class="recent__head">
                  <strong>{doc.title}</strong>
                  <span class="recent__when">{when(doc.at)}</span>
                </span>
                <span class="recent__excerpt">{doc.excerpt || copy.noText}</span>
              </button>
              <button
                class="recent__remove"
                aria-label={`Rimuovi ${doc.title} dall'elenco`}
                title="Rimuovi dall'elenco (il file non viene cancellato)"
                onclick={(event) => dropRecent(event, doc.path)}>×</button
              >
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section aria-labelledby="templates-title">
      <div class="home__section-head">
        <h2 id="templates-title" class="home__heading">{copy.templatesTitle}</h2>
        <input
          class="home__search"
          type="search"
          placeholder={copy.searchPlaceholder}
          aria-label={copy.searchLabel}
          bind:value={templateQuery}
        />
      </div>
      {#if filteredTemplates.length === 0}
        <p class="home__empty">
          {templates.length === 0 ? copy.templatesMissing : copy.templatesNoMatch}
        </p>
      {:else}
        <ul class="templates">
          {#each filteredTemplates as template (template.id)}
            <li class="template">
              <button class="template__open" onclick={() => onOpenTemplate(template.id)}>
                <span class="template__name">{template.name}</span>
                <span class="template__cat">{template.category}</span>
                <span class="template__desc">{template.description}</span>
              </button>
              <button
                class="template__star"
                aria-pressed={favorites.includes(template.id)}
                aria-label={favorites.includes(template.id)
                  ? `Rimuovi ${template.name} dai preferiti`
                  : `Aggiungi ${template.name} ai preferiti`}
                title="Modello preferito"
                onclick={() => star(template.id)}>{favorites.includes(template.id) ? "★" : "☆"}</button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </main>

  <footer class="home__footer">
    {#if stats}
      <span>{copy.footerStats(stats)}</span>
      <span>{copy.footerPacks(stats)}</span>
    {:else}
      <span>{copy.footerLicense}</span>
    {/if}
    {#if appInfo}
      <span class="home__version">v{appInfo.version} · {appInfo.platform}</span>
    {/if}
  </footer>
</div>

<style>
  .home {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 22px;
    padding: 26px clamp(16px, 4vw, 54px) 30px;
    background: var(--bg, #f7f8fa);
  }

  .home__top {
    display: flex;
    flex-wrap: wrap;
    gap: 14px;
    align-items: center;
    justify-content: space-between;
  }

  .home__brand {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .home__brand h1 {
    margin: 0;
    font-size: 1.5rem;
    letter-spacing: -0.02em;
  }

  .home__tagline {
    margin: 2px 0 0;
    font-size: 0.86rem;
    color: var(--muted, #5b6472);
  }

  .home__mark {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    border-radius: 14px;
    color: #fff;
    background: linear-gradient(140deg, #2f6fed, #1c4fbe);
    box-shadow: 0 6px 16px rgba(28, 79, 190, 0.28);
  }

  .home__mark :global(svg) {
    width: 30px;
    height: 30px;
  }

  .home__actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .home__main {
    display: flex;
    flex-direction: column;
    gap: 26px;
    max-width: 1180px;
    width: 100%;
    margin: 0 auto;
  }

  .home__heading {
    margin: 0 0 10px;
    font-size: 1.02rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted, #5b6472);
  }

  .home__section-head {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
    justify-content: space-between;
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(232px, 1fr));
    gap: 14px;
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-align: left;
    padding: 18px;
    border-radius: 16px;
    border: 1px solid var(--line, #d8dde6);
    background: var(--panel, #fff);
    color: inherit;
    cursor: pointer;
    transition: transform 0.12s ease, box-shadow 0.12s ease, border-color 0.12s ease;
    min-height: 176px;
  }

  .card:hover,
  .card:focus-visible {
    transform: translateY(-2px);
    border-color: #2f6fed;
    box-shadow: 0 10px 24px rgba(24, 39, 75, 0.12);
  }

  .card--main {
    border-color: #2f6fed;
    background: linear-gradient(170deg, #eef4ff, #fff 62%);
  }

  .card__icon {
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border-radius: 12px;
    background: #eaf0ff;
    color: #1c4fbe;
    margin-bottom: 4px;
  }

  .card__icon :global(svg) {
    width: 28px;
    height: 28px;
  }

  .card__title {
    font-weight: 700;
    font-size: 1.02rem;
  }

  .card__text {
    font-size: 0.88rem;
    line-height: 1.4;
  }

  .card__detail {
    margin-top: auto;
    font-size: 0.76rem;
    color: var(--muted, #5b6472);
  }

  .home__empty {
    margin: 0;
    font-size: 0.88rem;
    color: var(--muted, #5b6472);
  }

  .recent {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px;
  }

  .recent__item {
    position: relative;
    display: flex;
    border: 1px solid var(--line, #d8dde6);
    border-radius: 12px;
    background: var(--panel, #fff);
    overflow: hidden;
  }

  .recent__open {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
    text-align: left;
    background: none;
    border: 0;
    cursor: pointer;
    color: inherit;
  }

  .recent__head {
    display: flex;
    gap: 8px;
    align-items: baseline;
    justify-content: space-between;
  }

  .recent__when {
    font-size: 0.72rem;
    color: var(--muted, #5b6472);
    white-space: nowrap;
  }

  .recent__excerpt {
    font-size: 0.8rem;
    line-height: 1.35;
    color: var(--muted, #5b6472);
    display: -webkit-box;
    line-clamp: 3;
    -webkit-line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .recent__remove {
    width: 34px;
    border: 0;
    border-left: 1px solid var(--line, #d8dde6);
    background: none;
    color: var(--muted, #5b6472);
    cursor: pointer;
    font-size: 1rem;
  }

  .recent__remove:hover {
    background: #f3f5f9;
  }

  .templates {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 10px;
  }

  .template {
    display: flex;
    border: 1px solid var(--line, #d8dde6);
    border-radius: 12px;
    background: var(--panel, #fff);
    overflow: hidden;
  }

  .template__open {
    flex: 1;
    display: grid;
    gap: 2px;
    padding: 12px 14px;
    text-align: left;
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
  }

  .template__name {
    font-weight: 600;
  }

  .template__cat {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: #1c4fbe;
  }

  .template__desc {
    font-size: 0.8rem;
    color: var(--muted, #5b6472);
  }

  .template__star {
    width: 40px;
    border: 0;
    border-left: 1px solid var(--line, #d8dde6);
    background: none;
    font-size: 1.1rem;
    color: #b08300;
    cursor: pointer;
  }

  .template__star:hover {
    background: #fdf6e3;
  }

  .home__search {
    max-width: 260px;
  }

  .link-btn {
    background: none;
    border: 0;
    padding: 2px;
    font-size: 0.8rem;
    color: #1c4fbe;
    text-decoration: underline;
    cursor: pointer;
  }

  .home__footer {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-top: auto;
    padding-top: 10px;
    font-size: 0.78rem;
    color: var(--muted, #5b6472);
    border-top: 1px solid var(--line, #d8dde6);
  }

  .home__version {
    margin-left: auto;
  }
</style>
