<!-- ABOUTME: CodeMirror 6 editor component with DOT syntax highlighting. -->
<!-- ABOUTME: Emits debounced change events for live preview rendering. -->

<script lang="ts">
    import { onMount } from 'svelte';
    import { EditorState, Compartment, StateEffect, StateField, type Extension } from '@codemirror/state';
    import { EditorView, keymap, lineNumbers, highlightActiveLine, Decoration, type DecorationSet } from '@codemirror/view';
    import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
    import { bracketMatching } from '@codemirror/language';
    import { dotLanguage, dotHighlightStyle } from '$lib/dot-language';

    let {
        value = '',
        debounceMs = 300,
        wrap = false,
        onchange,
        oncursorchange,
    }: {
        value?: string;
        debounceMs?: number;
        wrap?: boolean;
        onchange?: (value: string) => void;
        oncursorchange?: (offset: number) => void;
    } = $props();

    let container: HTMLDivElement;
    let view: EditorView;
    let debounceTimer: ReturnType<typeof setTimeout>;
    let cursorDebounceTimer: ReturnType<typeof setTimeout>;
    const wrapCompartment = new Compartment();

    // Line decoration for statement highlighting (light grey background, like macOS app)
    const setHighlightEffect = StateEffect.define<{ from: number; to: number } | null>();
    const highlightLineDeco = Decoration.line({ class: 'cm-highlighted-line' });
    const highlightField = StateField.define<DecorationSet>({
        create() { return Decoration.none; },
        update(decos, tr) {
            for (const effect of tr.effects) {
                if (effect.is(setHighlightEffect)) {
                    if (!effect.value) return Decoration.none;
                    const { from, to } = effect.value;
                    const doc = tr.state.doc;
                    const startLine = doc.lineAt(from).number;
                    const endLine = doc.lineAt(Math.min(to, doc.length)).number;
                    const lines: any[] = [];
                    for (let i = startLine; i <= endLine; i++) {
                        lines.push(highlightLineDeco.range(doc.line(i).from));
                    }
                    return Decoration.set(lines);
                }
            }
            return decos;
        },
        provide: (f) => EditorView.decorations.from(f),
    });

    onMount(() => {
        const state = EditorState.create({
            doc: value,
            extensions: [
                lineNumbers(),
                highlightActiveLine(),
                bracketMatching(),
                history(),
                dotLanguage,
                dotHighlightStyle,
                wrapCompartment.of(wrap ? EditorView.lineWrapping : []),
                highlightField,
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

    $effect(() => {
        if (!view) return;
        view.dispatch({
            effects: wrapCompartment.reconfigure(wrap ? EditorView.lineWrapping : []),
        });
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

    // Select a range and scroll it into view
    export function selectRange(from: number, to: number) {
        if (!view) return;
        const clampedTo = Math.min(to, view.state.doc.length);
        view.dispatch({
            selection: { anchor: from, head: clampedTo },
            effects: EditorView.scrollIntoView(from, { y: 'center' }),
        });
        view.focus();
    }

    // Highlight a range with light grey background (no text selection)
    export function highlightRange(from: number, to: number) {
        if (!view) return;
        view.dispatch({
            effects: [
                setHighlightEffect.of({ from, to }),
                EditorView.scrollIntoView(from, { y: 'center' }),
            ],
        });
    }

    // Clear any line highlighting
    export function clearHighlight() {
        if (!view) return;
        view.dispatch({
            effects: setHighlightEffect.of(null),
        });
    }

    // Get current cursor offset in the document
    export function getCursorOffset(): number {
        return view?.state.selection.main.head ?? 0;
    }

    // Replace the entire document content
    export function setContent(content: string) {
        if (!view) return;
        view.dispatch({
            changes: { from: 0, to: view.state.doc.length, insert: content },
        });
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
    .editor-container :global(.cm-highlighted-line) {
        background-color: rgba(0, 0, 0, 0.06);
    }
</style>
