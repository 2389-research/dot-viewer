<!-- ABOUTME: Toolbar component with layout engine selector dropdown. -->
<!-- ABOUTME: Mirrors the macOS app's toolbar controls for the web editor. -->

<script lang="ts">
    let {
        engine = 'dot',
        onenginechange,
    }: {
        engine?: string;
        onenginechange?: (engine: string) => void;
    } = $props();

    const engines = ['dot', 'neato', 'fdp', 'circo', 'twopi', 'sfdp'];

    function handleChange(event: Event) {
        const target = event.target as HTMLSelectElement;
        onenginechange?.(target.value);
    }
</script>

<div class="toolbar">
    <label>
        Engine:
        <select value={engine} onchange={handleChange}>
            {#each engines as eng}
                <option value={eng}>{eng}</option>
            {/each}
        </select>
    </label>
</div>

<style>
    .toolbar {
        display: flex;
        align-items: center;
        gap: 12px;
        padding: 8px 16px;
        border-bottom: 1px solid #ddd;
        background: #fafafa;
    }
    label {
        font-size: 14px;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    select {
        padding: 4px 8px;
        border-radius: 4px;
        border: 1px solid #ccc;
    }
</style>
