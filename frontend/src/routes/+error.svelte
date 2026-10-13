<script lang="ts">
  // The error page: the status and its message, like the api's error
  // page (src/server/error.html).
  import { page } from '$app/state';

  import { errorMessage } from '#lib/errors.ts';

  let message = $derived(errorMessage(page.status));
</script>

<svelte:head>
  <title>{message}</title>
</svelte:head>

<div
  class="relative flex min-h-screen justify-center bg-white text-black antialiased sm:items-center sm:pt-0"
  role="main"
>
  <div class="mx-auto max-w-xl sm:px-6 lg:px-8">
    <div class="flex items-center pt-8 sm:justify-start sm:pt-0">
      <h1 class="border-r border-gray-400 px-4 text-lg text-black">{page.status}</h1>
      <div class="ml-4 text-lg text-black">{message}</div>
    </div>
  </div>
</div>
