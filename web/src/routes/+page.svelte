<!-- ABOUTME: Main page wiring the editor, preview, toolbar, and WASM module together. -->
<!-- ABOUTME: Implements debounced live preview with bidirectional editor-preview navigation. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import Editor from '$lib/components/Editor.svelte';
    import Preview from '$lib/components/Preview.svelte';
    import Toolbar from '$lib/components/Toolbar.svelte';
    import { renderDot, definitionRangeForNode, nodeIdAtOffset, parseDippin, type DippinSourceMapEntry } from '$lib/wasm';
    import type { Engine } from '@hpcc-js/wasm-graphviz';

    let svg = $state('');
    let error = $state('');
    let loading = $state(true);
    let engine: Engine = $state('dot');
    const initialSource = 'digraph G {\n    A -> B\n    B -> C\n    C -> A\n}';
    let currentSource = $state(initialSource);
    let highlightedNode: string | undefined = $state(undefined);
    let wrap = $state(false);
    let editor: Editor;

    let isDippin = $state(false);
    let generatedDot = $state(initialSource);
    let sourceMap = $state<DippinSourceMapEntry[]>([]);
    let parseError = $state('');

    let renderGeneration = 0;
    let interactionGeneration = 0;
    let parseGeneration = 0;

    onMount(async () => {
        await render(generatedDot);
        loading = false;
    });

    async function render(source: string) {
        const generation = ++renderGeneration;
        try {
            const result = await renderDot(source, engine);
            if (generation === renderGeneration) {
                svg = result;
                error = '';
            }
        } catch (e) {
            if (generation === renderGeneration) {
                error = e instanceof Error ? e.message : String(e);
            }
        }
    }

    async function handleEditorChange(value: string) {
        currentSource = value;
        if (isDippin) {
            const generation = ++parseGeneration;
            try {
                const result = await parseDippin(value);
                if (generation !== parseGeneration) return;
                generatedDot = result.dotSource;
                sourceMap = result.sourceMap;
                parseError = '';
            } catch (e) {
                if (generation !== parseGeneration) return;
                parseError = e instanceof Error ? e.message : String(e);
                error = parseError;
                return;
            }
        } else {
            generatedDot = value;
        }
        render(generatedDot);
    }

    function handleEngineChange(newEngine: string) {
        engine = newEngine as Engine;
        render(generatedDot);
    }

    function dotOffsetFromDip(dipOffset: number): number | null {
        if (!isDippin) return dipOffset;
        for (const e of sourceMap) {
            if (dipOffset >= e.dipStart && dipOffset < e.dipEnd) {
                return e.dotStart;
            }
        }
        return null;
    }

    function dipRangeFromDot(dotOffset: number): { start: number; end: number } | null {
        if (!isDippin) return { start: dotOffset, end: dotOffset };
        for (const e of sourceMap) {
            if (dotOffset >= e.dotStart && dotOffset < e.dotEnd) {
                return { start: e.dipStart, end: e.dipEnd };
            }
        }
        return null;
    }

    async function handleNodeClick(nodeId: string) {
        const generation = ++interactionGeneration;
        highlightedNode = nodeId;
        const range = await definitionRangeForNode(generatedDot, nodeId);
        if (generation !== interactionGeneration || !range) return;
        const dipRange = dipRangeFromDot(range.location);
        if (!dipRange) return;
        // Plain DOT identity yields a zero-length range; in that case fall back
        // to the full DOT range so we still select the whole statement.
        if (!isDippin) {
            editor.highlightRange(range.location, range.location + range.length);
        } else {
            editor.highlightRange(dipRange.start, dipRange.end);
        }
    }

    async function handleCursorChange(offset: number) {
        const generation = ++interactionGeneration;
        const dotOffset = dotOffsetFromDip(offset);
        if (dotOffset === null) {
            editor.clearHighlight();
            highlightedNode = undefined;
            return;
        }
        const nodeId = await nodeIdAtOffset(generatedDot, dotOffset);
        if (generation !== interactionGeneration) return;
        highlightedNode = nodeId;
        if (nodeId) {
            const range = await definitionRangeForNode(generatedDot, nodeId);
            if (generation === interactionGeneration && range) {
                if (!isDippin) {
                    editor.highlightRange(range.location, range.location + range.length);
                } else {
                    const dipRange = dipRangeFromDot(range.location);
                    if (dipRange) editor.highlightRange(dipRange.start, dipRange.end);
                }
            }
        } else {
            editor.clearHighlight();
        }
    }

    async function handleFileOpen(content: string, filename: string) {
        currentSource = content;
        editor.setContent(content);
        if (filename.endsWith('.dip')) {
            isDippin = true;
            const generation = ++parseGeneration;
            try {
                const result = await parseDippin(content);
                if (generation !== parseGeneration) return;
                generatedDot = result.dotSource;
                sourceMap = result.sourceMap;
                parseError = '';
            } catch (e) {
                if (generation !== parseGeneration) return;
                parseError = e instanceof Error ? e.message : String(e);
                error = parseError;
                return;
            }
        } else {
            // bumping the generation invalidates any in-flight dippin parse
            ++parseGeneration;
            isDippin = false;
            generatedDot = content;
            sourceMap = [];
            parseError = '';
        }
        render(generatedDot);
    }
</script>

<div class="app">
    <Toolbar {engine} {wrap} onenginechange={handleEngineChange} onfileopen={handleFileOpen} onwrapchange={(v) => wrap = v} />
    <div class="split-pane">
        <div class="editor-pane">
            <Editor
                bind:this={editor}
                value={currentSource}
                {wrap}
                onchange={handleEditorChange}
                oncursorchange={handleCursorChange}
            />
        </div>
        <div class="preview-pane">
            <Preview {svg} {error} {loading} onnodeclick={handleNodeClick} {highlightedNode} />
        </div>
    </div>
</div>

<style>
    .app {
        display: flex;
        flex-direction: column;
        height: 100vh;
    }
    .split-pane {
        display: flex;
        flex: 1;
        overflow: hidden;
    }
    .editor-pane {
        flex: 1;
        border-right: 1px solid #ddd;
        overflow: hidden;
    }
    .preview-pane {
        flex: 1;
        overflow: hidden;
    }
</style>
