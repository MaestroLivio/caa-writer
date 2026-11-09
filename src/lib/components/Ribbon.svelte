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
    onSettings: () => void;
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
    onSettings,
  }: Props = $props();

  let root: HTMLElement | undefined = $state();
  let width = $state(1440);
  let temporaryExpanded = $state(false);
  let bodyEl: HTMLDivElement | undefined = $state();
  let openGroup = $state<RibbonGroup | null>(null);
  let groupsMenuOpen = $state(false);
  let menuStyle = $state("");
  const currentGroups = $derived(
    tabs.find((tab) => tab.id === activeTab)?.groups ?? []
  );
  let hiddenFrom = $state(999);
  const groupWidths = new Map<string, number>();

  $effect(() => {
    // La larghezza della finestra è il dato affidabile: il contenuto del
    // Ribbon non deve poter allargare il layout oltre il viewport.
    const update = () => (width = window.innerWidth);
    update();
    window.addEventListener("resize", update);
    const observer = new ResizeObserver(update);
    if (root) observer.observe(root);
    return () => {
      window.removeEventListener("resize", update);
      observer.disconnect();
    };
  });

  // Sotto i 1200 px il Ribbon completo diventa compatto; sotto i 980 px si
  // nasconde, ma le schede restano e un clic le riapre temporaneamente.
  // Isteresi sui due confini: senza di essa un ridimensionamento pari a pochi
  // pixel attorno alla soglia faceva commutare continuamente modalità e
  // larghezze, producendo sfarfallio.
  let compactLatch = $state(false);
  let hiddenLatch = $state(false);
  $effect(() => {
    if (width < 1160) compactLatch = true;
    else if (width > 1240) compactLatch = false;
  });
  $effect(() => {
    if (width < 940) hiddenLatch = true;
    else if (width > 1020) hiddenLatch = false;
  });
  const autoCompact = $derived(compactLatch);
  const autoHidden = $derived(hiddenLatch);
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

  function placeMenu(event: MouseEvent, menuWidth: number) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const left = Math.max(8, Math.min(window.innerWidth - menuWidth - 8, rect.left));
    const top = Math.min(window.innerHeight - 120, rect.bottom + 4);
    menuStyle = `left:${Math.round(left)}px; top:${Math.round(top)}px; width:${menuWidth}px`;
  }

  function openMenu(group: RibbonGroup, event: MouseEvent) {
    placeMenu(event, 300);
    groupsMenuOpen = false;
    openGroup = group;
    void tick().then(() => {
      const first = document.querySelector<HTMLElement>(".ribbon-menu button, .ribbon-menu select");
      first?.focus();
    });
  }

  function openGroupsMenu(event: MouseEvent) {
    placeMenu(event, 320);
    openGroup = null;
    groupsMenuOpen = true;
    void tick().then(() => {
      const first = document.querySelector<HTMLElement>(".ribbon-menu button, .ribbon-menu select");
      first?.focus();
    });
  }

  function closeMenu() {
    openGroup = null;
    groupsMenuOpen = false;
  }

  function availableWidth(): number {
    if (!bodyEl) return 140;
    return Math.max(140, bodyEl.clientWidth);
  }

  /// Calcola quanti gruppi entrano usando le larghezze misurate. Prima prova
  /// senza menu «Gruppi (n)»; se non basta riserva lo spazio del menu. Non
  /// tocca il DOM: durante il ridimensionamento non si torna mai a mostrare
  /// tutti i gruppi (era questa la causa dello sfarfallio su Windows).
  function computeHiddenFrom(): number {
    const groups = currentGroups;
    if (groups.length === 0) return 0;
    const widths = groups.map((group) => groupWidths.get(group.id) ?? 190);
    const available = availableWidth();
    const total = widths.reduce((sum, value) => sum + value, 0);
    if (total <= available) return groups.length;
    const menuReserve = 112;
    let used = 0;
    for (let index = 0; index < widths.length; index += 1) {
      if (used + widths[index] + menuReserve > available && index > 0) {
        return index;
      }
      used += widths[index];
    }
    return Math.max(1, groups.length - 1);
  }

  /// Misura i gruppi renderizzati una volta per scheda/modalità e aggiorna la
  /// cache. Le misurazioni successive usano solo la cache.
  function measureGroups() {
    if (!bodyEl) return;
    const groups = currentGroups;
    if (groups.length === 0) {
      hiddenFrom = 0;
      return;
    }
    for (const group of groups) {
      const element = bodyEl.querySelector<HTMLElement>(
        `[data-group-id="${group.id}"]`
      );
      const measured = element?.getBoundingClientRect().width ?? 0;
      if (measured > 0) groupWidths.set(group.id, measured);
    }
    hiddenFrom = computeHiddenFrom();
  }

  // Identità strutturale dei gruppi: gli aggiornamenti di stato dei comandi
  // (attivo/disattivo) non devono invalidare le misure e far ricomparire tutti
  // i gruppi per un frame.
  const groupsSignature = $derived(
    currentGroups
      .map((group) => `${group.id}:${group.commands.length}:${group.visible ?? "auto"}`)
      .join("|")
  );

  const visibleGroups = $derived(currentGroups.slice(0, hiddenFrom));
  const hiddenGroups = $derived(currentGroups.slice(hiddenFrom));

  // Scheda o modalità cambiate: servono nuove misure perché cambiano le
  // larghezze dei gruppi (etichette e controlli compatti).
  $effect(() => {
    void activeTab;
    void effectiveMode;
    void groupsSignature;
    groupWidths.clear();
    hiddenFrom = 999;
    const frame = requestAnimationFrame(measureGroups);
    return () => cancelAnimationFrame(frame);
  });

  // Ridimensionamento: si usano le larghezze già misurate. Nessun reset a 999,
  // quindi i gruppi non ricompaiono per un frame e non si genera il flicker.
  $effect(() => {
    void width;
    if (!bodyEl || currentGroups.length === 0) return;
    if (groupWidths.size < currentGroups.length) return;
    const next = computeHiddenFrom();
    if (next !== hiddenFrom) hiddenFrom = next;
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

{#snippet commandTile(command: RibbonCommand)}
  {@const Icon = command.icon ? ribbonIcons[command.icon as keyof typeof ribbonIcons] : null}
  <button
    type="button"
    class="ribbon-tile"
    class:ribbon-tile--active={command.active}
    aria-pressed={command.active === undefined ? undefined : command.active}
    aria-label={command.label}
    title={command.tooltip ?? command.label}
    disabled={command.enabled === false}
    onclick={() => run(command)}
  >
    <span class="ribbon-tile__icon" aria-hidden="true">
      {#if Icon}<RibbonIcon node={Icon} size={24} />{/if}
    </span>
    <span class="ribbon-tile__label">{command.label}</span>
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

    <button
      class="btn btn--ghost ribbon__settings"
      onclick={onSettings}
      title="Impostazioni"
      aria-label="Apri le impostazioni"
    >⚙</button>

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

  {#if effectiveMode !== "hidden" || temporaryExpanded}
    <div class="ribbon__bodywrap">
      <div
        class="ribbon__body"
        class:ribbon__body--switch={switching}
        bind:this={bodyEl}
        tabindex="0"
        role="toolbar"
        aria-label="Comandi della scheda attiva"
      >
        {#each visibleGroups as group (group.id)}
          <section
            class="ribbon-group"
            class:ribbon-group--tiles={group.layout === "tiles"}
            data-group-id={group.id}
            aria-label={group.label}
          >
            <div class="ribbon-group__commands">
              {#each visibleCommands(group) as command (command.id)}
                {#if group.layout === "tiles" && command.kind !== "select" && command.kind !== "color"}
                  {@render commandTile(command)}
                {:else}
                  {@render control(command)}
                {/if}
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
        {#if hiddenGroups.length > 0}
          <button
            class="ribbon-btn ribbon-btn--more ribbon-btn--groups"
            aria-label={`Altri gruppi di comandi (${hiddenGroups.length})`}
            title={`Gruppi non visibili per mancanza di spazio (${hiddenGroups.length})`}
            aria-haspopup="menu"
            onclick={openGroupsMenu}
          >
            <RibbonIcon node={ribbonIcons.ellipsis} size={17} />
            <span class="ribbon-btn__label">Gruppi ({hiddenGroups.length})</span>
          </button>
        {/if}
      </div>
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
  {#if groupsMenuOpen}
    <button class="ribbon-menu-backdrop" aria-label="Chiudi il menu" onclick={closeMenu}></button>
    <div class="ribbon-menu ribbon-menu--groups" style={menuStyle} role="menu" aria-label="Altri gruppi di comandi">
      <p class="ribbon-menu__title">Gruppi non visibili</p>
      {#each hiddenGroups as group (group.id)}
        <div class="ribbon-menu__group">
          <p class="ribbon-menu__subtitle">{group.label}</p>
          {#each group.commands as command (command.id)}
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
      {/each}
    </div>
  {/if}
  {/if}
</div>
