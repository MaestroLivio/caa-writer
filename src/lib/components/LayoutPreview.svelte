<script lang="ts">
  import type { LayoutDocument } from "../types";

  let { doc, compact = false }: { doc: LayoutDocument; compact?: boolean } = $props();
</script>

<div class="layout-pages" class:compact>
  {#each doc.pages as page (page.index)}
    <article
      class="layout-page"
      style={`width:${page.width_mm}mm;height:${page.height_mm}mm`}
      aria-label={`Pagina ${page.index + 1} di ${doc.pages.length}`}
    >
      {#each page.elements.filter((element) => !element.hidden) as element (element.id)}
        <div
          class="layout-el"
          class:locked={element.locked}
          style={`left:${element.x_mm}mm;top:${element.y_mm}mm;width:${element.w_mm}mm;height:${element.h_mm}mm;font-size:${element.font_mm}mm`}
        >
          {#if element.image}
            <div class="layout-el__symbol"><img src={element.image} alt="" /></div>
          {:else if element.svg}
            <div class="layout-el__symbol">{@html element.svg}</div>
          {/if}
          {#if element.label}
            <div class="layout-el__label">{element.label}</div>
          {/if}
        </div>
      {/each}
    </article>
  {/each}
</div>

<style>
  .layout-pages {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 4px 0;
  }

  .layout-page {
    position: relative;
    background: #ffffff;
    color: #111111;
    box-shadow: 0 2px 14px rgba(0, 0, 0, 0.18);
    overflow: hidden;
    box-sizing: border-box;
    flex: 0 0 auto;
    break-after: page;
  }

  .layout-page:last-child {
    break-after: auto;
  }

  .layout-el {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.6mm;
    padding: 1mm;
    border: 0.3mm solid #2b2b2b;
    border-radius: 1.5mm;
    overflow: hidden;
    text-align: center;
    line-height: 1.05;
  }

  .layout-el.locked {
    border-style: double;
    border-width: 0.6mm;
  }

  .layout-el__symbol {
    flex: 1 1 auto;
    min-height: 0;
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .layout-el__symbol :global(svg),
  .layout-el__symbol :global(img) {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .layout-el__label {
    flex: 0 0 auto;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Anteprima ridotta: la geometria resta in millimetri, cambia solo la scala
     visiva del contenitore. */
  .compact .layout-page {
    zoom: 0.42;
  }

  @media print {
    .layout-pages {
      gap: 0;
      padding: 0;
    }

    .layout-page {
      box-shadow: none;
      margin: 0;
      page-break-after: always;
      break-after: page;
    }

    .compact .layout-page {
      zoom: 1;
    }
  }
</style>
