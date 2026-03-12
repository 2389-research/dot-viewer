<!-- ABOUTME: SVG preview component with pan/zoom and clickable nodes. -->
<!-- ABOUTME: Renders DOT output as inline SVG with bidirectional navigation support. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import panzoom from 'panzoom';

    let {
        svg = '',
        error = '',
        loading = false,
        onnodeclick,
        highlightedNode,
    }: {
        svg?: string;
        error?: string;
        loading?: boolean;
        onnodeclick?: (nodeId: string) => void;
        highlightedNode?: string;
    } = $props();

    let container: HTMLDivElement;
    let svgContainer: HTMLDivElement;

    // Apply or remove the 'highlighted' class on the matching SVG node group
    function updateHighlight(nodeId: string | undefined) {
        if (!svgContainer) return;
        // Clear any existing highlights
        for (const el of svgContainer.querySelectorAll('.node.highlighted')) {
            el.classList.remove('highlighted');
        }
        if (!nodeId) return;
        // Find the node group whose <title> matches the nodeId
        for (const node of svgContainer.querySelectorAll('.node')) {
            const title = node.querySelector('title');
            if (title?.textContent === nodeId) {
                node.classList.add('highlighted');
                return;
            }
        }
    }

    $effect(() => {
        // Re-run when svg or highlightedNode changes
        svg;
        // Use a microtask so the SVG DOM has been updated by Svelte
        queueMicrotask(() => updateHighlight(highlightedNode));
    });
    let panzoomInstance: ReturnType<typeof panzoom> | null = null;

    onMount(() => {
        if (svgContainer) {
            panzoomInstance = panzoom(svgContainer, {
                maxZoom: 10,
                minZoom: 0.1,
                smoothScroll: false,
            });
        }

        return () => {
            panzoomInstance?.dispose();
        };
    });

    function handleClick(event: MouseEvent) {
        let el = event.target as Element | null;
        while (el && el !== svgContainer) {
            if (el.classList?.contains('node')) {
                const title = el.querySelector('title');
                if (title?.textContent) {
                    onnodeclick?.(title.textContent);
                    return;
                }
            }
            el = el.parentElement;
        }
    }
</script>

<div class="preview-container" bind:this={container}>
    {#if loading}
        <div class="loading">Loading Graphviz...</div>
    {/if}

    {#if error}
        <div class="error-bar">{error}</div>
    {/if}

    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
    <div
        class="svg-container"
        bind:this={svgContainer}
        onclick={handleClick}
        role="img"
    >
        {@html svg}
    </div>
</div>

<style>
    .preview-container {
        height: 100%;
        position: relative;
        overflow: hidden;
        background: white;
    }
    .svg-container {
        width: 100%;
        height: 100%;
    }
    .loading {
        position: absolute;
        top: 50%;
        left: 50%;
        transform: translate(-50%, -50%);
        color: #666;
    }
    .error-bar {
        position: absolute;
        bottom: 0;
        left: 0;
        right: 0;
        background: #fee;
        color: #c00;
        padding: 8px 16px;
        font-size: 13px;
        z-index: 1;
    }
    .svg-container :global(.node.highlighted ellipse),
    .svg-container :global(.node.highlighted polygon) {
        stroke: #e6a817;
        stroke-width: 3px;
    }
</style>
