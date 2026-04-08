// ABOUTME: End-to-end tests for opening .dip files and bidirectional linking.
// ABOUTME: Verifies dippin parsing, preview rendering, and node click highlighting.

import { test, expect } from '@playwright/test';
// @ts-expect-error - node builtin, types not required for playwright test runner
import path from 'node:path';
// @ts-expect-error - node builtin, types not required for playwright test runner
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const fixtureDir = path.join(here, 'fixtures');

test('opening a .dip file renders the converted DOT graph', async ({ page }) => {
    await page.goto('/');
    // Wait for default graph to render so we know the wasm is loaded.
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(fixtureDir, 'sample.dip'));

    // After opening, the SVG should re-render with the dippin-derived graph.
    // Wait briefly for the parse + render pipeline.
    await page.waitForTimeout(500);
    await expect(page.locator('.svg-container svg')).toBeVisible();
    const nodeCount = await page.locator('.svg-container .node').count();
    expect(nodeCount).toBeGreaterThanOrEqual(2);
});

test('clicking a rendered node highlights dippin source', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(path.join(fixtureDir, 'sample.dip'));
    await page.waitForTimeout(500);

    // Click the first SVG node and verify the editor shows a highlighted
    // block in the dippin source.
    const firstNode = page.locator('.svg-container .node').first();
    await firstNode.click();
    await page.waitForTimeout(300);

    // The highlighted-line decoration should appear in the editor.
    await expect(page.locator('.cm-editor .cm-highlighted-line').first()).toBeVisible();
});
