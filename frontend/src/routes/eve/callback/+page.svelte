<script lang="ts">
  import AuthToken from '#lib/components/auth-token.svelte';
  import PageHead from '#lib/components/page-head.svelte';

  let { data } = $props();
</script>

<PageHead title="Authentication" />

<AuthToken token={data.token} deepLink={data.deepLink} characterName={data.characterName} />
