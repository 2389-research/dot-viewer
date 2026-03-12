<!-- ABOUTME: Main page wiring the editor, preview, toolbar, and WASM module together. -->
<!-- ABOUTME: Implements debounced live preview with bidirectional editor-preview navigation. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import Editor from '$lib/components/Editor.svelte';
    import Preview from '$lib/components/Preview.svelte';
    import Toolbar from '$lib/components/Toolbar.svelte';
    import { renderDot, definitionOffsetForNode, nodeIdAtOffset } from '$lib/wasm';
    import type { Engine } from '@hpcc-js/wasm-graphviz';

    let svg = $state('');
    let error = $state('');
    let loading = $state(true);
    let engine: Engine = $state('dot');
    let currentSource = $state('digraph G {\n    A -> B\n    B -> C\n    C -> A\n}');
    let highlightedNode: string | undefined = $state(undefined);
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
        const offset = await definitionOffsetForNode(currentSource, nodeId);
        if (offset !== undefined) {
            editor.scrollToOffset(offset);
        }
    }

    async function handleCursorChange(offset: number) {
        highlightedNode = await nodeIdAtOffset(currentSource, offset);
    }
</script>

<div class="app">
    <Toolbar {engine} onenginechange={handleEngineChange} />
    <div class="split-pane">
        <div class="editor-pane">
            <Editor
                bind:this={editor}
                value={currentSource}
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
