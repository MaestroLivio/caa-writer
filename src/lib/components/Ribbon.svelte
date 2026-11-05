<script lang="ts">
  import RibbonIcon from "./RibbonIcon.svelte";
  import { ribbonIcons } from "../ribbon/icons";
  import type { RibbonCommand, RibbonMode, RibbonTab } from "../ribbon/types";

  interface Props {
    tabs: RibbonTab[];
    activeTab: string;
    onTab: (id: string) => void;
    fileCommands: RibbonCommand[];
    quickCommands: RibbonCommand[];
    quickOptions: { id: string; label: string }[];
    quickAccess: string[];
    onToggleQuick: (id: string) => void;
    onHome: () => void;
    docTitle: string;
    docPath: string | null;
    dirty: boolean;
    savedAt: string;
    mode: RibbonMode;
    onMode: (mode: RibbonMode) => void;
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
    onHome,
    docTitle,
    docPath,
    dirty,
    savedAt,
    mode,
    onMode,
  }: Props = $props();

  let root: HTMLElement | undefined = $state();
  let width = $state(1440);
  let temporaryExpanded = $state(false);

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
  const effectiveMode = $derived(
    temporaryExpanded
      ? "full"
      : mode === "full" && autoCompact
        ? "compact"
        : mode === "compact" && autoHidden
          ? "hidden"
          : mode
  );

  function selectTab(id: string) {
    onTab(id);
    if (effectiveMode === "hidden") temporaryExpanded = true;
  }

  function run(command: RibbonCommand) {
    if (command.enabled === false) return;
    command.execute?.();
  }
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

<div class="ribbon" bind:this={root} data-mode={effectiveMode}>
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
    <div class="ribbon__body">
      {#each tabs.filter((tab) => tab.id === activeTab) as tab (tab.id)}
        {#each tab.groups as group (group.id)}
          <section class="ribbon-group" aria-label={group.label}>
            <div class="ribbon-group__commands">
              {#each group.commands as command (command.id)}
                {@render control(command)}
              {/each}
            </div>
            <div class="ribbon-group__label">{group.label}</div>
          </section>
        {/each}
      {/each}
    </div>
  {/if}
</div>
