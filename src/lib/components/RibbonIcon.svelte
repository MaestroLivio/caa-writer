<script lang="ts">
  import type { IconNode } from "lucide";

  interface Props {
    node: IconNode;
    size?: number;
    strokeWidth?: number;
  }

  let { node, size = 18, strokeWidth = 1.9 }: Props = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
  focusable="false"
>
  {#each node as [tag, attrs]}
    <svelte:element this={tag} {...attrs} />
  {/each}
</svg>
