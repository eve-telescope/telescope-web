<script lang="ts">
  // The desktop login's last page: open the app
  // with the token, or copy it.
  import { CopyState } from '#lib/opener.svelte.ts';

  let {
    token,
    deepLink,
    characterName,
  }: { token: string; deepLink: string; characterName: string } = $props();

  const copy = new CopyState();

  function copyToken() {
    copy.copy(token, (text) => navigator.clipboard.writeText(text));
  }

  function openInApp() {
    window.location.href = deepLink;
  }
</script>

<div class="flex min-h-screen items-center justify-center bg-eve-bg-0 p-4">
  <div class="w-full max-w-md space-y-6 text-center">
    <div class="mb-2 text-4xl text-eve-cyan">&#x2713;</div>

    <h1 class="text-xl font-bold tracking-wider text-eve-text-1">
      Authenticated as {characterName}
    </h1>

    <p class="text-sm text-eve-text-2">
      Click below to open Telescope, or copy the token to paste it manually.
    </p>

    <button
      class="w-full cursor-pointer rounded bg-eve-cyan py-3 text-sm font-bold tracking-wider text-eve-bg-0 transition-all hover:bg-eve-cyan-dim"
      onclick={openInApp}
    >
      OPEN IN TELESCOPE
    </button>

    <div class="space-y-2">
      <p class="text-[10px] tracking-widest text-eve-text-3">OR COPY TOKEN</p>
      <button
        type="button"
        class="flex w-full cursor-pointer items-center gap-2 rounded border border-eve-border bg-eve-bg-2 px-3 py-2 transition-colors hover:border-eve-cyan"
        onclick={copyToken}
      >
        <code class="flex-1 truncate text-left text-xs text-eve-text-2">{token}</code>
        <span class="shrink-0 text-[10px] text-eve-cyan">
          {copy.copied ? 'COPIED!' : 'COPY'}
        </span>
      </button>
    </div>
  </div>
</div>
