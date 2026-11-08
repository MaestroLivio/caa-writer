<script lang="ts">
  import RibbonIcon from "./RibbonIcon.svelte";
  import { ribbonIcons } from "../ribbon/icons";
  import type { WorkspacePanel } from "../workspace/layout";

  interface Props {
    activePanel: WorkspacePanel;
    onOpen: (panel: WorkspacePanel) => void;
  }

  let { activePanel, onOpen }: Props = $props();

  const items: {
    id: WorkspacePanel;
    label: string;
    short: string;
    icon: keyof typeof ribbonIcons;
  }[] = [
    { id: "simbolo", label: "Simbolo: alternative e ricerca", short: "Simbolo", icon: "search" },
    { id: "collezioni", label: "Collezioni personali", short: "Coll.", icon: "library" },
    { id: "studio", label: "Visual Resource Studio", short: "Studio", icon: "layout-grid" },
  ];
</script>

<aside class="sidebar-rail" aria-label="Pannello laterale compresso">
  {#each items as item (item.id)}
    <button
      type="button"
      class="sidebar-rail__button"
      class:sidebar-rail__button--active={activePanel === item.id}
      aria-label={`Apri ${item.label}`}
      title={item.label}
      onclick={() => onOpen(item.id)}
    >
      <RibbonIcon node={ribbonIcons[item.icon]} size={19} />
      <span>{item.short}</span>
    </button>
  {/each}
  <button
    type="button"
    class="sidebar-rail__expand"
    aria-label="Espandi il pannello laterale"
    title="Espandi il pannello laterale"
    onclick={() => onOpen(activePanel)}
  >»</button>
</aside>
