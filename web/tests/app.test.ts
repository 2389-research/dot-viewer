// ABOUTME: End-to-end tests for the Dot Viewer web app.
// ABOUTME: Verifies editor, preview, toolbar controls, and bidirectional navigation.

import { test, expect } from '@playwright/test';

test('page loads with editor and preview', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.editor-container')).toBeVisible();
    await expect(page.locator('.preview-container')).toBeVisible();
});

test('default graph renders SVG on load', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const nodes = page.locator('.svg-container .node');
    await expect(nodes).toHaveCount(3); // A, B, C from default graph
});

test('editor has syntax highlighting', async ({ page }) => {
    await page.goto('/');
    // The keyword "digraph" should be colored (purple, not default black)
    const digraphSpan = page.locator('.cm-editor .cm-line span', { hasText: 'digraph' }).first();
    await expect(digraphSpan).toBeVisible();
    const color = await digraphSpan.evaluate((el) => getComputedStyle(el).color);
    expect(color).toBe('rgb(175, 82, 222)'); // systemPurple
});

test('editor accepts text input and re-renders', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const editor = page.locator('.cm-editor .cm-content');
    await editor.click();
    await page.keyboard.press('ControlOrMeta+A');
    await page.keyboard.type('digraph T { X -> Y }');
    await page.waitForTimeout(500);
    const svg = page.locator('.svg-container svg');
    await expect(svg).toBeVisible();
    // New graph should have X and Y nodes
    const nodeTexts = await page.locator('.svg-container .node title').allTextContents();
    expect(nodeTexts).toContain('X');
    expect(nodeTexts).toContain('Y');
});

test('engine picker changes layout', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const select = page.locator('.toolbar select');
    await select.selectOption('neato');
    await page.waitForTimeout(500);
    await expect(page.locator('.svg-container svg')).toBeVisible();
});

test('invalid DOT shows error bar', async ({ page }) => {
    await page.goto('/');
    const editor = page.locator('.cm-editor .cm-content');
    await editor.click();
    await page.keyboard.press('ControlOrMeta+A');
    await page.keyboard.type('not valid dot {{{');
    await page.waitForTimeout(500);
    await expect(page.locator('.error-bar')).toBeVisible();
});

test('clicking SVG node highlights block in editor', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    // Click on the first node in the SVG
    const node = page.locator('.svg-container .node').first();
    await node.click();
    await page.waitForTimeout(300);
    // Editor should show highlighted line(s) with grey background
    await expect(page.locator('.cm-editor .cm-highlighted-line')).toBeVisible();
});

test('clicking SVG node highlights it with blue', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const node = page.locator('.svg-container .node').first();
    await node.click();
    await page.waitForTimeout(300);
    await expect(page.locator('.svg-container .node.highlighted')).toHaveCount(1);
});

test('SVG nodes show pointer cursor', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    const node = page.locator('.svg-container .node').first();
    const cursor = await node.evaluate((el) => getComputedStyle(el).cursor);
    expect(cursor).toBe('pointer');
});

test('SVG node shapes accept pointer events for full-shape clicking', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    // Ellipses (and polygons) should have pointer-events: all so clicks
    // register on the whole shape, not just the stroke
    const ellipse = page.locator('.svg-container .node ellipse').first();
    const pointerEvents = await ellipse.evaluate((el) => getComputedStyle(el).pointerEvents);
    expect(pointerEvents).toBe('all');
});

test('editor highlight clears when clicking outside nodes', async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('.svg-container svg', { timeout: 10000 });
    // First highlight a node
    const node = page.locator('.svg-container .node').first();
    await node.click();
    await page.waitForTimeout(300);
    await expect(page.locator('.cm-editor .cm-highlighted-line')).toBeVisible();
    // Click on the first line ("digraph G {") which is not a node statement
    const firstLine = page.locator('.cm-editor .cm-line').first();
    await firstLine.click();
    await page.waitForTimeout(500);
    // Highlight should clear since cursor is on "digraph G {" line, not a node
    await expect(page.locator('.cm-editor .cm-highlighted-line')).toHaveCount(0);
});

test('wrap toggle enables line wrapping', async ({ page }) => {
    await page.goto('/');
    const wrapCheckbox = page.locator('.wrap-toggle input[type="checkbox"]');
    await expect(wrapCheckbox).not.toBeChecked();
    await wrapCheckbox.check();
    await expect(wrapCheckbox).toBeChecked();
    // CodeMirror should now have the line-wrapping class
    await expect(page.locator('.cm-editor .cm-lineWrapping')).toBeVisible();
});

test('toolbar has open button', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.toolbar button', { hasText: 'Open' })).toBeVisible();
});
