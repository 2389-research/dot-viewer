// @ts-nocheck
// ABOUTME: Structural tests for +page.svelte dippin wiring (Task 14).
// ABOUTME: Runs without vitest - parses the source and asserts on key invariants.

import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const pageSrc = readFileSync(resolve(here, '+page.svelte'), 'utf-8');
const wasmSrc = readFileSync(resolve(here, '../lib/wasm.ts'), 'utf-8');

let pass = 0;
let fail = 0;
function check(name: string, cond: boolean) {
    if (cond) { pass++; console.log('  ok', name); }
    else { fail++; console.log('  FAIL', name); }
}

console.log('+page.svelte dippin wiring:');
check('imports parseDippin from $lib/wasm', /import\s*{[^}]*parseDippin[^}]*}\s*from\s*['"]\$lib\/wasm['"]/.test(pageSrc));
check('declares isDippin state', /let\s+isDippin\s*=\s*\$state\(/.test(pageSrc));
check('declares generatedDot state', /let\s+generatedDot\s*=\s*\$state\(/.test(pageSrc));
check('declares sourceMap state', /let\s+sourceMap\s*=\s*\$state/.test(pageSrc));
check('declares parseError state', /let\s+parseError\s*=\s*\$state\(/.test(pageSrc));
check('handleFileOpen branches on .dip extension', /filename\.endsWith\(['"]\.dip['"]\)/.test(pageSrc));
check('handleEditorChange re-parses dippin', /handleEditorChange[\s\S]*?if\s*\(isDippin\)[\s\S]*?parseDippin/.test(pageSrc));
check('uses parseGeneration race guard', /parseGeneration/.test(pageSrc));
check('renderer uses generatedDot not currentSource', /render\(generatedDot\)/.test(pageSrc));
check('declares dotOffsetFromDip helper', /function\s+dotOffsetFromDip/.test(pageSrc));
check('declares dipRangeFromDot helper', /function\s+dipRangeFromDot/.test(pageSrc));
check('handleCursorChange uses dotOffsetFromDip', /handleCursorChange[\s\S]*?dotOffsetFromDip/.test(pageSrc));
check('handleNodeClick uses dipRangeFromDot', /handleNodeClick[\s\S]*?dipRangeFromDot/.test(pageSrc));
check('TODO(T15) marker removed', !/TODO\(T15\)/.test(pageSrc));

console.log('wasm.ts dippin exports:');
check('exports DippinConversion interface', /export\s+interface\s+DippinConversion/.test(wasmSrc));
check('exports parseDippin function', /export\s+async\s+function\s+parseDippin/.test(wasmSrc));
check('parseDippin awaits ensureInit', /parseDippin[\s\S]*?await\s+ensureInit/.test(wasmSrc));

console.log(`\n${pass} passed, ${fail} failed`);
if (fail > 0) process.exit(1);
