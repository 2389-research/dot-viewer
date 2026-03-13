<!-- ABOUTME: Main page wiring the editor, preview, toolbar, and WASM module together. -->
<!-- ABOUTME: Implements debounced live preview with bidirectional editor-preview navigation. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import Editor from '$lib/components/Editor.svelte';
    import Preview from '$lib/components/Preview.svelte';
    import Toolbar from '$lib/components/Toolbar.svelte';
    import { renderDot, definitionRangeForNode, nodeIdAtOffset } from '$lib/wasm';
    import type { Engine } from '@hpcc-js/wasm-graphviz';

    let svg = $state('');
    let error = $state('');
    let loading = $state(true);
    let engine: Engine = $state('dot');
    let currentSource = $state('digraph G {\n    A -> B\n    B -> C\n    C -> A\n}');
    let highlightedNode: string | undefined = $state(undefined);
    let wrap = $state(false);
    let editor: Editor;

    let renderGeneration = 0;

    onMount(async () => {
        await render(currentSource);
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

    function handleEditorChange(value: string) {
        currentSource = value;
        render(currentSource);
    }

    function handleEngineChange(newEngine: string) {
        engine = newEngine as Engine;
        render(currentSource);
    }

    async function handleNodeClick(nodeId: string) {
        highlightedNode = nodeId;
        const range = await definitionRangeForNode(currentSource, nodeId);
        if (range) {
            editor.highlightRange(range.location, range.location + range.length);
        }
    }

    async function handleCursorChange(offset: number) {
        const nodeId = await nodeIdAtOffset(currentSource, offset);
        highlightedNode = nodeId;
        if (nodeId) {
            const range = await definitionRangeForNode(currentSource, nodeId);
            if (range) {
                editor.highlightRange(range.location, range.location + range.length);
            }
        } else {
            editor.clearHighlight();
        }
    }

    function handleFileOpen(content: string) {
        currentSource = content;
        editor.setContent(content);
        render(currentSource);
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
