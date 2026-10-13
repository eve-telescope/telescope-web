<script lang="ts">
  // The animated starfield behind the pages: random stars, placed in the
  // browser once the page is mounted, as the Vue pages did.
  import { onMount } from 'svelte';

  let { count, maxOpacity }: { count: number; maxOpacity: number } = $props();

  type Star = { x: number; y: number; size: number; opacity: number; delay: number };

  let stars = $state<Star[]>([]);

  onMount(() => {
    stars = Array.from({ length: count }, () => ({
      x: Math.random() * 100,
      y: Math.random() * 100,
      size: Math.random() * 2 + 0.5,
      opacity: Math.random() * maxOpacity + 0.2,
      delay: Math.random() * 3,
    }));
  });
</script>

<div class="absolute inset-0 overflow-hidden" data-starfield>
  {#each stars as star, i (i)}
    <div
      class="absolute animate-pulse rounded-full bg-white"
      style:left="{star.x}%"
      style:top="{star.y}%"
      style:width="{star.size}px"
      style:height="{star.size}px"
      style:opacity={star.opacity}
      style:animation-delay="{star.delay}s"
      style:animation-duration="3s"
    ></div>
  {/each}
</div>
