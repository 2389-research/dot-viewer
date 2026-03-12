<!-- ABOUTME: Root layout component wrapping all pages. -->
<!-- ABOUTME: Sets up the favicon and renders child route content. -->

<script lang="ts">
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{@render children()}
