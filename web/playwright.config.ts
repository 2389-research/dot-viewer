// ABOUTME: Playwright configuration for E2E testing the Dot Viewer web app.
// ABOUTME: Tests against the built static site using Chromium.

import { defineConfig } from '@playwright/test';

export default defineConfig({
    testDir: 'tests',
    webServer: {
        command: 'npm run preview',
        port: 4173,
        reuseExistingServer: true,
    },
    use: {
        baseURL: 'http://localhost:4173',
    },
});
