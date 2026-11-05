<script lang="ts">
  import { tick } from "svelte";
  import RibbonIcon from "./RibbonIcon.svelte";
  import { ribbonIcons } from "../ribbon/icons";
  import type { RibbonCommand, RibbonGroup, RibbonMode, RibbonTab } from "../ribbon/types";

  interface Props {
    tabs: RibbonTab[];
    activeTab: string;
    onTab: (id: string) => void;
    fileCommands: RibbonCommand[];
    quickCommands: RibbonCommand[];
    quickOptions: { id: string; label: string }[];
    quickAccess: string[];
    onToggleQuick: (id: string) => void;
    quickLabels: boolean;
    onToggleQuickLabels: () => void;
    onHome: () => void;
    docTitle: string;
    docPath: string | null;
    dirty: boolean;
    savedAt: string;
    mode: RibbonMode;
    onMode: (mode: RibbonMode) => void;
    pinned: boolean;
    onTogglePin: () => void;
    topbarMinimized: boolean;
    onToggleTopbar: () => void;
  }

  let {
    tabs,
    activeTab,
    onTab,
    fileCommands,
    quickCommands,
    quickOptions,
    quickAccess,
    onToggleQuick,
    quickLabels,
    onToggleQuickLabels,
    onHome,
    docTitle,
    docPath,
    dirty,
    savedAt,
    mode,
    onMode,
    pinned,
    onTogglePin,
    topbarMinimized,
    onToggleTopbar,
  }: Props = $props();

  let root: HTMLElement | undefined = $state();
  let width = $state(1440);
  let temporaryExpanded = $state(false);
  let bodyEl: HTMLDivElement | undefined = $state();
  let scrollLeft = $state(0);
  let bodyWidth = $state(0);
  let bodyScrollWidth = $state(0);
  let openGroup = $state<RibbonGroup | null>(null);
  let menuStyle = $state("");

  $effect(() => {
    if (!root) return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (entry) width = entry.contentRect.width;
    });
    observer.observe(root);
    return () => observer.disconnect();
  });

  // Sotto i 1200 px il Ribbon completo diventa compatto; sotto i 980 px si
  // nasconde, ma le schede restano e un clic le riapre temporaneamente.
  const autoCompact = $derived(width < 1200);
  const autoHidden = $derived(width < 980);
  const overlaid = $derived(
    (mode === "hidden" && (temporaryExpanded || pinned)) ||
      (mode === "compact" && autoHidden && temporaryExpanded)
  );
  const effectiveMode = $derived(
    overlaid
      ? "full"
      : mode === "full" && autoCompact
        ? "compact"
        : mode === "compact" && autoHidden
          ? "hidden"
          : mode
  );

  let switching = $state(false);

  function selectTab(id: string) {
    onTab(id);
    switching = true;
    window.setTimeout(() => (switching = false), 220);
    if (effectiveMode === "hidden") temporaryExpanded = true;
  }

  // In modalità minimizzata il corpo è sovrapposto: un clic nel documento lo
  // chiude, senza spostare il contenuto. Se l'utente ha bloccato il Ribbon con
  // il doppio clic, resta aperto.
  $effect(() => {
    if (!overlaid || pinned) return;
    const onDown = (event: MouseEvent) => {
      if (root && !root.contains(event.target as Node)) temporaryExpanded = false;
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  });

  function run(command: RibbonCommand) {
    if (command.enabled === false) return;
    command.execute?.();
  }

  function visibleCount(group: RibbonGroup): number {
    const requested = group.visible ?? (effectiveMode === "compact" ? 3 : 5);
    return effectiveMode === "compact" ? Math.min(requested, 3) : requested;
  }

  function visibleCommands(group: RibbonGroup): RibbonCommand[] {
    return group.commands.slice(0, visibleCount(group));
  }

  function overflowCommands(group: RibbonGroup): RibbonCommand[] {
    return group.commands.slice(visibleCount(group));
  }

  function updateScroll() {
    if (!bodyEl) return;
    scrollLeft = bodyEl.scrollLeft;
    bodyWidth = bodyEl.clientWidth;
    bodyScrollWidth = bodyEl.scrollWidth;
  }

  function scrollBy(delta: number) {
    bodyEl?.scrollBy({ left: delta, behavior: "smooth" });
  }

  function openMenu(group: RibbonGroup, event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const menuWidth = 300;
    const left = Math.max(8, Math.min(window.innerWidth - menuWidth - 8, rect.left));
    const top = Math.min(window.innerHeight - 80, rect.bottom + 4);
    menuStyle = `left:${Math.round(left)}px; top:${Math.round(top)}px; width:${menuWidth}px`;
    openGroup = group;
    void tick().then(() => {
      const first = document.querySelector<HTMLElement>(".ribbon-menu button, .ribbon-menu select");
      first?.focus();
    });
  }

  function closeMenu() {
    openGroup = null;
  }

  const canScrollLeft = $derived(scrollLeft > 2);
  const canScrollRight = $derived(scrollLeft + bodyWidth < bodyScrollWidth - 2);

  $effect(() => {
    // Ricalcola la disponibilità di scorrimento quando cambiano scheda,
    // modalità, larghezza o numero di comandi.
    void activeTab;
    void effectiveMode;
    void width;
    void tabs;
    requestAnimationFrame(updateScroll);
  });

  $effect(() => {
    if (!openGroup) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        closeMenu();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

{#snippet commandButton(command: RibbonCommand)}
  {@const Icon = command.icon ? ribbonIcons[command.icon as keyof typeof ribbonIcons] : null}
  <button
    type="button"
    class="ribbon-btn"
    class:ribbon-btn--primary={command.emphasis === "primary"}
    class:ribbon-btn--active={command.active}
    class:ribbon-btn--compact={effectiveMode === "compact"}
    aria-pressed={command.active === undefined ? undefined : command.active}
    aria-label={command.label}
    title={command.tooltip ?? `${command.label}${command.shortcut ? ` (${command.shortcut})` : ""}`}
    disabled={command.enabled === false}
    onclick={() => run(command)}
  >
    {#if Icon}<RibbonIcon node={Icon} size={command.emphasis === "primary" ? 20 : 16} />{/if}
    <span class="ribbon-btn__label">{command.label}</span>
    {#if command.shortcut && command.emphasis === "primary"}
      <span class="ribbon-btn__shortcut">{command.shortcut}</span>
    {/if}
  </button>
{/snippet}

{#snippet control(command: RibbonCommand)}
  {#if command.kind === "select"}
    <label class="ribbon-control">
      <span>{command.label}</span>
      <select
        aria-label={command.label}
        title={command.tooltip ?? command.label}
        value={command.value}
        disabled={command.enabled === false}
        onchange={(event) => command.onChange?.((event.currentTarget as HTMLSelectElement).value)}
        style={command.width ? `width:${command.width}` : ""}
      >
        {#each command.options ?? [] as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    </label>
  {:else if command.kind === "color"}
    <label class="ribbon-control ribbon-control--color">
      <span>{command.label}</span>
      <input
        type="color"
        aria-label={command.label}
        title={command.tooltip ?? command.label}
        value={command.value ?? "#000000"}
        onchange={(event) => command.onChange?.((event.currentTarget as HTMLInputElement).value)}
      />
    </label>
  {:else}
    {@render commandButton(command)}
  {/if}
{/snippet}

<div
  class="ribbon"
  class:ribbon--overlay={overlaid}
  bind:this={root}
  data-mode={effectiveMode}
  data-quick-labels={quickLabels ? "true" : "false"}
>
  {#if topbarMinimized}
    <div
      class="ribbon__slim"
      role="toolbar"
      aria-label="Barra superiore minimizzata"
      tabindex="0"
      ondblclick={onToggleTopbar}
      title="Barra superiore minimizzata: doppio clic per ripristinarla"
    >
      <button
        class="btn btn--ghost"
        aria-label="Ripristina la barra superiore"
        title="Ripristina la barra superiore"
        onclick={onToggleTopbar}>⌄</button
      >
      <strong>{tabs.find((tab) => tab.id === activeTab)?.label ?? "CAA Writer"}</strong>
      <div class="ribbon__quick" role="group" aria-label="Accesso rapido">
        {#each quickCommands as command (command.id)}
          {@render commandButton({ ...command, emphasis: "secondary" })}
        {/each}
      </div>
      <button class="ribbon__slim-show" onclick={onToggleTopbar}>Mostra barra</button>
    </div>
  {:else}
  <div class="ribbon__docbar">
    <button class="btn btn--ghost ribbon__home" onclick={onHome} title="Torna alla schermata iniziale" aria-label="Torna alla schermata iniziale">⌂</button>
    <div class="ribbon__docmeta">
      <strong class="ribbon__title">{docTitle}{dirty ? " •" : ""}</strong>
      <span class="ribbon__status" title={docPath ?? docTitle}>
        {dirty ? "Modifiche non salvate" : "Salvato"}
        {#if savedAt}<span class="hint"> · bozza {savedAt}</span>{/if}
      </span>
    </div>

    <div class="ribbon__file" role="group" aria-label="File">
      {#each fileCommands as command (command.id)}
        {@render commandButton({ ...command, emphasis: "secondary" })}
      {/each}
    </div>

    <div class="ribbon__quick" role="group" aria-label="Accesso rapido">
      {#each quickCommands as command (command.id)}
        {@render commandButton({ ...command, emphasis: "secondary" })}
      {/each}
      <details class="ribbon__customize">
        <summary class="btn btn--ghost" title="Personalizza i comandi di accesso rapido" aria-label="Personalizza accesso rapido">
          <span aria-hidden="true">⋯</span>
        </summary>
        <div class="ribbon__customize-panel">
          <p>Comandi sempre disponibili</p>
          <label>
            <input type="checkbox" checked={quickLabels} onchange={onToggleQuickLabels} />
            Mostra le etichette
          </label>
          {#each quickOptions as option (option.id)}
            <label>
              <input
                type="checkbox"
                checked={quickAccess.includes(option.id)}
                onchange={() => onToggleQuick(option.id)}
              />
              {option.label}
            </label>
          {/each}
        </div>
      </details>
    </div>

    <div class="ribbon__modes" role="group" aria-label="Modalità del Ribbon">
      <button
        class="btn btn--ghost"
        class:btn--solid={mode === "full"}
        aria-pressed={mode === "full"}
        title="Ribbon completo"
        onclick={() => { temporaryExpanded = false; onMode("full"); }}>▤</button
      >
      <button
        class="btn btn--ghost"
        class:btn--solid={mode === "compact"}
        aria-pressed={mode === "compact"}
        title="Ribbon compatto"
        onclick={() => { temporaryExpanded = false; onMode("compact"); }}>▥</button
      >
      <button
        class="btn btn--ghost"
        class:btn--solid={mode === "hidden"}
        aria-pressed={mode === "hidden"}
        title="Nascondi il Ribbon"
        onclick={() => { temporaryExpanded = false; onMode("hidden"); }}>▦</button
      >
      <button
        class="btn btn--ghost"
        title="Minimizza l'intera barra superiore"
        aria-label="Minimizza l'intera barra superiore"
        onclick={onToggleTopbar}>⌃</button
      >
    </div>
  </div>

  <div class="ribbon__tabs" role="tablist" aria-label="Schede del Ribbon">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        role="tab"
        class="ribbon__tab"
        class:ribbon__tab--contextual={tab.contextual}
        aria-selected={activeTab === tab.id}
        onclick={() => selectTab(tab.id)}
        ondblclick={(event) => {
          event.preventDefault();
          onTogglePin();
        }}
        title={`${tab.label} · doppio clic per bloccare o sbloccare il Ribbon`}
      >{tab.label}</button>
    {/each}
    {#if effectiveMode === "hidden"}
      <button class="ribbon__expand" onclick={() => (temporaryExpanded = true)} title="Mostra i comandi" aria-label="Mostra i comandi">
        <span aria-hidden="true">⌄</span> Comandi
      </button>
    {:else if temporaryExpanded}
      <button class="ribbon__expand" onclick={() => (temporaryExpanded = false)} title="Riduci il Ribbon" aria-label="Riduci il Ribbon">
        <span aria-hidden="true">⌃</span>
      </button>
    {/if}
  </div>

  {#if effectiveMode !== "hidden"}
    <div class="ribbon__bodywrap">
      {#if canScrollLeft}
        <button
          class="ribbon__scroll ribbon__scroll--left"
          aria-label="Scorri il Ribbon a sinistra"
          title="Scorri a sinistra"
          onclick={() => scrollBy(-180)}>‹</button
        >
      {/if}
      <div
        class="ribbon__body"
        class:ribbon__body--switch={switching}
        bind:this={bodyEl}
        onscroll={updateScroll}
        onwheel={(event) => {
          if (bodyScrollWidth <= bodyWidth) return;
          const delta = event.deltaX || event.deltaY;
          if (delta) {
            event.preventDefault();
            bodyEl!.scrollLeft += delta;
          }
        }}
        onkeydown={(event) => {
          if (event.key === "ArrowLeft") {
            event.preventDefault();
            scrollBy(-120);
          } else if (event.key === "ArrowRight") {
            event.preventDefault();
            scrollBy(120);
          } else if (event.key === "Home") {
            event.preventDefault();
            bodyEl?.scrollTo({ left: 0, behavior: "smooth" });
          } else if (event.key === "End") {
            event.preventDefault();
            bodyEl?.scrollTo({ left: bodyScrollWidth, behavior: "smooth" });
          }
        }}
        tabindex="0"
        role="toolbar"
        aria-label="Comandi della scheda attiva"
      >
        {#each tabs.filter((tab) => tab.id === activeTab) as tab (tab.id)}
          {#each tab.groups as group (group.id)}
            <section class="ribbon-group" aria-label={group.label}>
              <div class="ribbon-group__commands">
                {#each visibleCommands(group) as command (command.id)}
                  {@render control(command)}
                {/each}
                {#if overflowCommands(group).length > 0}
                  <button
                    class="ribbon-btn ribbon-btn--more"
                    aria-label={`Altri comandi: ${group.label}`}
                    title={`Altri comandi di ${group.label} (${overflowCommands(group).length})`}
                    aria-haspopup="menu"
                    onclick={(event) => openMenu(group, event)}
                  >
                    <RibbonIcon node={ribbonIcons.ellipsis} size={17} />
                    <span class="ribbon-btn__label">Altri</span>
                  </button>
                {/if}
              </div>
              <div class="ribbon-group__label">{group.label}</div>
            </section>
          {/each}
        {/each}
      </div>
      {#if canScrollRight}
        <button
          class="ribbon__scroll ribbon__scroll--right"
          aria-label="Scorri il Ribbon a destra"
          title="Scorri a destra"
          onclick={() => scrollBy(180)}>›</button
        >
      {/if}
    </div>
  {/if}
  {#if openGroup}
    <button class="ribbon-menu-backdrop" aria-label="Chiudi il menu" onclick={closeMenu}></button>
    <div class="ribbon-menu" style={menuStyle} role="menu" aria-label={`Altri comandi: ${openGroup.label}`}>
      <p class="ribbon-menu__title">{openGroup.label}</p>
      {#each overflowCommands(openGroup) as command (command.id)}
        {#if command.kind === "select" || command.kind === "color"}
          {@render control(command)}
        {:else}
          {@const Icon = command.icon ? ribbonIcons[command.icon as keyof typeof ribbonIcons] : null}
          <button
            type="button"
            class="ribbon-menu__item"
            role="menuitem"
            disabled={command.enabled === false}
            title={command.tooltip ?? command.label}
            onclick={() => {
              run(command);
              closeMenu();
            }}
          >
            {#if Icon}<RibbonIcon node={Icon} size={16} />{/if}
            <span>{command.label}</span>
            {#if command.shortcut}<span class="ribbon-menu__shortcut">{command.shortcut}</span>{/if}
          </button>
        {/if}
      {/each}
    </div>
  {/if}
  {/if}
</div>
