<!-- ABOUTME: CodeMirror 6 editor component with DOT syntax support. -->
<!-- ABOUTME: Emits debounced change events for live preview rendering. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import { EditorState } from '@codemirror/state';
    import { EditorView, keymap, lineNumbers, highlightActiveLine } from '@codemirror/view';
    import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
    import { bracketMatching } from '@codemirror/language';

    let {
        value = '',
        debounceMs = 300,
        onchange,
        oncursorchange,
    }: {
        value?: string;
        debounceMs?: number;
        onchange?: (value: string) => void;
        oncursorchange?: (offset: number) => void;
    } = $props();

    let container: HTMLDivElement;
    let view: EditorView;
    let debounceTimer: ReturnType<typeof setTimeout>;
    let cursorDebounceTimer: ReturnType<typeof setTimeout>;

    onMount(() => {
        const state = EditorState.create({
            doc: value,
            extensions: [
                lineNumbers(),
                highlightActiveLine(),
                bracketMatching(),
                history(),
                keymap.of([...defaultKeymap, ...historyKeymap]),
                EditorView.updateListener.of((update) => {
                    if (update.docChanged) {
                        const newValue = update.state.doc.toString();
                        clearTimeout(debounceTimer);
                        debounceTimer = setTimeout(() => {
                            onchange?.(newValue);
                        }, debounceMs);
                    }
                    if (update.selectionSet) {
                        const offset = update.state.selection.main.head;
                        clearTimeout(cursorDebounceTimer);
                        cursorDebounceTimer = setTimeout(() => {
                            oncursorchange?.(offset);
                        }, debounceMs);
                    }
                }),
            ],
        });

        view = new EditorView({ state, parent: container });

        return () => {
            clearTimeout(debounceTimer);
            clearTimeout(cursorDebounceTimer);
            view?.destroy();
        };
    });

    // Set cursor to a specific document offset and focus the editor
    export function setCursorPosition(offset: number) {
        if (!view) return;
        view.dispatch({ selection: { anchor: offset } });
        view.focus();
    }

    // Scroll to a specific document offset, centering it in the viewport
    export function scrollToOffset(offset: number) {
        if (!view) return;
        view.dispatch({
            selection: { anchor: offset },
            effects: EditorView.scrollIntoView(offset, { y: 'center' }),
        });
        view.focus();
    }

    // Get current cursor offset in the document
    export function getCursorOffset(): number {
        return view?.state.selection.main.head ?? 0;
    }
</script>

<div bind:this={container} class="editor-container"></div>

<style>
    .editor-container {
        height: 100%;
        overflow: auto;
    }
    .editor-container :global(.cm-editor) {
        height: 100%;
    }
</style>
