// ABOUTME: End-to-end tests for opening .dip files and bidirectional linking.
// ABOUTME: Verifies dippin parsing, preview rendering, and node click highlighting.

import { test, expect } from '@playwright/test';
// @ts-expect-error - node builtin, types not required for playwright test runner
import path from 'node:path';
// @ts-expect-error - node builtin, types not required for playwright test runner
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixtureDir = path.join(here, 'fixtures');

// The default graph contains nodes A/B/C, which collide with our fixture's
// A/B. To prove the .dip actually loaded we wait for an editor line that only
// the dippin fixture produces ("workflow Hello") before asserting on the SVG.
async function openSampleDip(page: import('@playwright/test').Page) {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(fixtureDir, 'sample.dip'));
    // Editor mirrors the raw dippin source — wait until the fixture's first
    // line is visible so we know parseDippin + setContent have run.
    await expect(
        page.locator('.cm-editor .cm-line', { hasText: 'workflow Hello' }).first(),
    ).toBeVisible({ timeout: 5000 });
}

test('opening a .dip file renders the converted DOT graph', async ({ page }) => {
    await openSampleDip(page);
    await expect(page.locator('.svg-container svg')).toBeVisible();
    // sample.dip declares exactly two agents (A, B); the converted DOT must
    // produce exactly those two nodes — distinct from the default A/B/C graph.
    await expect(page.locator('.svg-container .node')).toHaveCount(2);
});

test('clicking a rendered node highlights dippin source', async ({ page }) => {
    await openSampleDip(page);
    const firstNode = page.locator('.svg-container .node').first();
    await firstNode.click();
    // The highlighted-line decoration should appear in the editor.
    await expect(page.locator('.cm-editor .cm-highlighted-line').first()).toBeVisible();
});
