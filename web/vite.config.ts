// ABOUTME: Vite configuration for the Dot Viewer SvelteKit app.
// ABOUTME: Integrates the SvelteKit Vite plugin for development and builds.

import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()]
});
