<script lang="ts">
  // Tastiera virtuale flottante della modalità Utilizzatore.
  //
  // Parte ridotta a un piccolo indicatore: si apre solo su richiesta, si può
  // trascinare per non coprire la frase, si ridimensiona dal bordo e ricorda
  // posizione e dimensioni al riavvio. I tasti sono ampi per il tocco.
  import { onMount } from "svelte";

  interface Props {
    /// Profilo corrente: geometria separata per ogni utilizzatore.
    profileId: string;
    onInsert: (text: string) => void;
    onSpace: () => void;
    onEnter: () => void;
    onBackspace: () => void;
  }

  let { profileId, onInsert, onSpace, onEnter, onBackspace }: Props = $props();

  const letterRows = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"].map((row) => row.split(""));
  const accentRow = ["à", "è", "é", "ì", "ò", "ù", "ç", "'"];

  type Geometry = { x: number; y: number; w: number; h: number };
  const MIN_W = 320;
  const MIN_H = 220;

  let open = $state(false);
  let geometry = $state<Geometry>({ x: 0, y: 0, w: 460, h: 300 });
  let loaded = $state(false);
  let dragging = $state(false);
  let dragOffset = { x: 0, y: 0 };
  let resizing = $state(false);
  let sizeStart = { x: 0, y: 0, w: 0, h: 0 };

  const storageKey = $derived(`caa.keyboard.${profileId}`);

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function defaultGeometry(): Geometry {
    const viewportW = typeof window === "undefined" ? 1024 : window.innerWidth;
    const viewportH = typeof window === "undefined" ? 768 : window.innerHeight;
    const w = Math.min(480, Math.max(MIN_W, viewportW - 48));
    const h = Math.min(340, Math.max(MIN_H, viewportH - 160));
    return { x: Math.max(12, viewportW - w - 24), y: Math.max(12, viewportH - h - 96), w, h };
  }

  function load() {
    try {
      const raw = localStorage.getItem(storageKey);
      if (!raw) {
        geometry = defaultGeometry();
        return;
      }
      const parsed = JSON.parse(raw) as Partial<Geometry>;
      const next = {
        x: Number(parsed.x),
        y: Number(parsed.y),
        w: Number(parsed.w),
        h: Number(parsed.h),
      };
      if (![next.x, next.y, next.w, next.h].every(Number.isFinite)) {
        geometry = defaultGeometry();
        return;
      }
      geometry = clampGeometry(next);
    } catch {
      geometry = defaultGeometry();
    }
  }

  function clampGeometry(value: Geometry): Geometry {
    const viewportW = typeof window === "undefined" ? 1024 : window.innerWidth;
    const viewportH = typeof window === "undefined" ? 768 : window.innerHeight;
    const w = clamp(Math.round(value.w), MIN_W, Math.max(MIN_W, viewportW - 24));
    const h = clamp(Math.round(value.h), MIN_H, Math.max(MIN_H, viewportH - 96));
    return {
      w,
      h,
      x: clamp(Math.round(value.x), 8, Math.max(8, viewportW - w - 8)),
      y: clamp(Math.round(value.y), 8, Math.max(8, viewportH - h - 24)),
    };
  }

  function persist() {
    try {
      localStorage.setItem(storageKey, JSON.stringify(geometry));
    } catch {
      /* spazio locale pieno: la geometria vale per la sessione */
    }
  }

  onMount(() => {
    load();
    loaded = true;
    const onResize = () => {
      if (loaded) geometry = clampGeometry(geometry);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  });

  function startDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    dragging = true;
    dragOffset = { x: event.clientX - geometry.x, y: event.clientY - geometry.y };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveDrag(event: PointerEvent) {
    if (!dragging) return;
    geometry = clampGeometry({
      ...geometry,
      x: event.clientX - dragOffset.x,
      y: event.clientY - dragOffset.y,
    });
  }

  function endDrag(event: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (event.currentTarget as HTMLElement).releasePointerCapture?.(event.pointerId);
    persist();
  }

  function startResize(event: PointerEvent) {
    event.stopPropagation();
    resizing = true;
    sizeStart = { x: event.clientX, y: event.clientY, w: geometry.w, h: geometry.h };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveResize(event: PointerEvent) {
    if (!resizing) return;
    const viewportW = window.innerWidth;
    const viewportH = window.innerHeight;
    geometry = clampGeometry({
      ...geometry,
      w: clamp(sizeStart.w + event.clientX - sizeStart.x, MIN_W, Math.max(MIN_W, viewportW - geometry.x - 8)),
      h: clamp(sizeStart.h + event.clientY - sizeStart.y, MIN_H, Math.max(MIN_H, viewportH - geometry.y - 24)),
    });
  }

  function endResize(event: PointerEvent) {
    if (!resizing) return;
    resizing = false;
    (event.currentTarget as HTMLElement).releasePointerCapture?.(event.pointerId);
    persist();
  }

  /// La tastiera si può usare anche con le frecce: Invio ed Esc restano del
  /// documento, quindi i tasti non intercettano le scorciatoie globali.
  function key(label: string) {
    onInsert(label);
  }
</script>

{#if open}
  <section
    class="fkb"
    style={`left:${geometry.x}px;top:${geometry.y}px;width:${geometry.w}px;height:${geometry.h}px;`}
    aria-label="Tastiera virtuale"
    role="dialog"
  >
    <header
      class="fkb__bar"
      onpointerdown={startDrag}
      onpointermove={moveDrag}
      onpointerup={endDrag}
      onpointercancel={endDrag}
      title="Trascina per spostare la tastiera"
    >
      <span class="fkb__title">Tastiera</span>
      <div class="fkb__bar-actions">
        <button
          class="fkb__icon"
          onclick={() => {
            open = false;
            persist();
          }}
          aria-label="Riduci la tastiera"
          title="Riduci"
        >–</button>
      </div>
    </header>

    <div class="fkb__keys">
      {#each letterRows as row, rowIndex (rowIndex)}
        <div class="fkb__row">
          {#each row as letter (letter)}
            <button class="fkb__key" type="button" onclick={() => key(letter)}>{letter}</button>
          {/each}
        </div>
      {/each}
      <div class="fkb__row">
        {#each accentRow as letter (letter)}
          <button class="fkb__key" type="button" onclick={() => key(letter)}>{letter}</button>
        {/each}
      </div>
      <div class="fkb__row">
        <button class="fkb__key fkb__key--wide" type="button" onclick={onSpace}>Spazio</button>
        <button class="fkb__key fkb__key--wide" type="button" onclick={onEnter}>Invio</button>
        <button class="fkb__key fkb__key--wide" type="button" onclick={onBackspace}>Cancella</button>
      </div>
    </div>

    <button
      class="fkb__resize"
      type="button"
      aria-label="Ridimensiona la tastiera"
      title="Trascina per ridimensionare"
      onpointerdown={startResize}
      onpointermove={moveResize}
      onpointerup={endResize}
      onpointercancel={endResize}
    ></button>
  </section>
{:else}
  <button
    class="fkb__indicator"
    type="button"
    onclick={() => {
      open = true;
      geometry = clampGeometry(geometry);
    }}
    aria-label="Apri la tastiera virtuale"
    title="Apri la tastiera virtuale"
  >
    <span aria-hidden="true">⌨</span>
  </button>
{/if}

<style>
  .fkb {
    position: fixed;
    z-index: 120;
    display: flex;
    flex-direction: column;
    min-width: 320px;
    min-height: 220px;
    background: var(--surface, #fff);
    border: 1px solid var(--border, #cfd8e3);
    border-radius: 14px;
    box-shadow: 0 18px 44px rgba(15, 23, 42, 0.25);
    overflow: hidden;
    touch-action: none;
  }
  .fkb__bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 8px 6px 12px;
    background: var(--surface-2, #eef2f8);
    border-bottom: 1px solid var(--border, #cfd8e3);
    cursor: move;
    user-select: none;
  }
  .fkb__title { font-size: 0.82rem; font-weight: 700; }
  .fkb__bar-actions { display: flex; gap: 4px; }
  .fkb__icon {
    width: 40px;
    height: 36px;
    border: 1px solid var(--border, #cfd8e3);
    background: var(--surface, #fff);
    border-radius: 9px;
    font-size: 1.1rem;
    line-height: 1;
    cursor: pointer;
    touch-action: manipulation;
  }
  .fkb__keys {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    overflow: auto;
  }
  .fkb__row { display: flex; gap: 5px; justify-content: center; }
  .fkb__key {
    flex: 1 1 0;
    min-width: 30px;
    min-height: 44px;
    border: 1px solid var(--border, #cfd8e3);
    border-radius: 9px;
    background: var(--surface, #fff);
    color: var(--ink, #17202a);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    touch-action: manipulation;
  }
  .fkb__key:hover { background: var(--accent-soft, #dbe7ff); }
  .fkb__key:active { transform: translateY(1px); }
  .fkb__key--wide {
    flex: 1.6 1 0;
    font-size: 0.82rem;
    background: var(--surface-2, #eef2f8);
  }
  .fkb__resize {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 22px;
    height: 22px;
    border: none;
    background:
      linear-gradient(135deg, transparent 0 45%, var(--border, #cfd8e3) 45% 55%, transparent 55% 100%);
    cursor: nwse-resize;
    touch-action: none;
  }
  .fkb__indicator {
    position: fixed;
    right: 18px;
    bottom: 18px;
    z-index: 110;
    width: 44px;
    height: 44px;
    padding: 0;
    border-radius: 50%;
    border: 1px solid var(--border, #cfd8e3);
    background: var(--surface, #fff);
    box-shadow: 0 8px 20px rgba(15, 23, 42, 0.22);
    font-size: 1rem;
    line-height: 1;
    cursor: pointer;
    touch-action: manipulation;
  }
  .fkb__indicator:hover { background: var(--accent-soft, #dbe7ff); }
</style>
