// ABOUTME: Vite configuration for the Dot Viewer SvelteKit app.
// ABOUTME: Integrates the SvelteKit Vite plugin and configures WASM serving.

import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [sveltekit()],
	optimizeDeps: {
		exclude: ['dot-core-wasm']
	},
	server: {
		fs: {
			allow: ['..']
		}
	}
});
