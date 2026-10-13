<script lang="ts">
  // The page's <head>: its title and the description, OpenGraph and
  // Twitter tags (seo.ts).
  import { page } from '$app/state';

  import { pageHead } from '#lib/seo.ts';

  let { title }: { title?: string } = $props();

  let head = $derived(pageHead(page.url, title));
</script>

<svelte:head>
  <title>{head.title}</title>
  {#each head.meta as tag (tag.name ?? tag.property)}
    {#if tag.name}
      <meta name={tag.name} content={tag.content} />
    {:else}
      <meta property={tag.property} content={tag.content} />
    {/if}
  {/each}
</svelte:head>
