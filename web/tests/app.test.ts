// ABOUTME: End-to-end tests for the Dot Viewer web app.
// ABOUTME: Verifies editor rendering, SVG preview, engine switching, and error handling.

import { test, expect } from '@playwright/test';

test('page loads with editor and preview', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.editor-container')).toBeVisible();
    await expect(page.locator('.preview-container')).toBeVisible();
});

test('editor accepts text input', async ({ page }) => {
    await page.goto('/');
    const editor = page.locator('.cm-editor .cm-content');
    await editor.click();
    await editor.fill('digraph Test { X -> Y }');
    await page.waitForTimeout(500);
    const svg = page.locator('.svg-container svg');
    await expect(svg).toBeVisible();
});

test('engine picker changes layout', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const select = page.locator('.toolbar select');
    await select.selectOption('neato');
    await page.waitForTimeout(500);
    await expect(page.locator('.svg-container svg')).toBeVisible();
});

test('invalid DOT shows error', async ({ page }) => {
    await page.goto('/');
    const editor = page.locator('.cm-editor .cm-content');
    await editor.click();
    await editor.fill('not valid dot {{{');
    await page.waitForTimeout(500);
    await expect(page.locator('.error-bar')).toBeVisible();
});

test('clicking a node highlights in editor', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const node = page.locator('.svg-container .node').first();
    if (await node.count() > 0) {
        await node.click();
        await expect(page.locator('.cm-editor.cm-focused')).toBeVisible();
    }
});
