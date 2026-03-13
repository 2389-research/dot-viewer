<!-- ABOUTME: Toolbar component with file open button and layout engine selector. -->
<!-- ABOUTME: Mirrors the macOS app's toolbar controls for the web editor. -->

<script lang="ts">
    let {
        engine = 'dot',
        wrap = false,
        onenginechange,
        onfileopen,
        onwrapchange,
    }: {
        engine?: string;
        wrap?: boolean;
        onenginechange?: (engine: string) => void;
        onfileopen?: (content: string) => void;
        onwrapchange?: (wrap: boolean) => void;
    } = $props();

    const engines = ['dot', 'neato', 'fdp', 'circo', 'twopi', 'sfdp'];

    let fileInput: HTMLInputElement;

    function handleChange(event: Event) {
        const target = event.target as HTMLSelectElement;
        onenginechange?.(target.value);
    }

    function handleOpenClick() {
        fileInput.click();
    }

    async function handleFileSelected(event: Event) {
        const target = event.target as HTMLInputElement;
        const file = target.files?.[0];
        if (!file) return;
        const content = await file.text();
        onfileopen?.(content);
        // Reset so the same file can be re-opened
        target.value = '';
    }
</script>

<div class="toolbar">
    <button onclick={handleOpenClick}>Open</button>
    <input
        bind:this={fileInput}
        type="file"
        accept=".dot,.gv,.txt"
        onchange={handleFileSelected}
        hidden
    />
    <label class="wrap-toggle">
        <input type="checkbox" checked={wrap} onchange={(e) => onwrapchange?.((e.currentTarget as HTMLInputElement).checked)} />
        Wrap
    </label>
    <label>
        Engine:
        <select value={engine} onchange={handleChange}>
            {#each engines as eng}
                <option value={eng} selected={eng === engine}>{eng}</option>
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
    button {
        padding: 4px 12px;
        border-radius: 4px;
        border: 1px solid #ccc;
        background: white;
        cursor: pointer;
        font-size: 14px;
    }
    button:hover {
        background: #f0f0f0;
    }
    label {
        font-size: 14px;
        display: flex;
        align-items: center;
        gap: 6px;
    }
    .wrap-toggle {
        cursor: pointer;
        user-select: none;
    }
    select {
        padding: 4px 8px;
        border-radius: 4px;
        border: 1px solid #ccc;
    }
</style>
