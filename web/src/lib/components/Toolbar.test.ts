// ABOUTME: Structural tests for Toolbar.svelte asserting .dip support and filename callback.
// ABOUTME: Parses the component source directly since vitest/testing-library are not installed.

// @ts-nocheck
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(__dirname, 'Toolbar.svelte'), 'utf-8');

function assert(cond: boolean, msg: string): void {
    if (!cond) {
        console.error(`FAIL: ${msg}`);
        process.exit(1);
    }
    console.log(`PASS: ${msg}`);
}

// Test 1: accept attribute includes .dip
const acceptMatch = source.match(/accept="([^"]+)"/);
assert(acceptMatch !== null, 'Toolbar has an accept attribute');
assert(
    acceptMatch![1].split(',').map((s) => s.trim()).includes('.dip'),
    'Toolbar accept attribute includes .dip',
);

// Test 2: onfileopen type signature includes filename: string
assert(
    /onfileopen\?:\s*\(content:\s*string,\s*filename:\s*string\)\s*=>\s*void/.test(source),
    'Toolbar onfileopen type signature accepts (content, filename)',
);

// Test 3: onfileopen invocation passes file.name alongside content
assert(
    /onfileopen\?\.\(content,\s*file\.name\)/.test(source),
    'Toolbar invokes onfileopen with file.name',
);

console.log('All Toolbar structural tests passed.');
