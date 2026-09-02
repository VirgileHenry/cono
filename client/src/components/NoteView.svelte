<script lang="ts">
    import { tick } from "svelte";
    import type { Note } from "../lib/protocol";
    import { openNoteId } from "../lib/stores";
    import {
        editor,
        openEditor,
        closeEditor,
        onLocalInput,
    } from "../lib/editor";

    let { note }: { note: Note } = $props();

    let textarea: HTMLTextAreaElement;
    let lastExternalChange = 0;

    /* Open the editor on mount / when switching notes. */
    $effect(() => {
        openEditor(note);
        return () => closeEditor();
    });

    /* When content changed from outside the textarea (remote op, resync),
     * svelte re-renders the value; restore/adjust the cursor after. */
    $effect(() => {
        const state = $editor;
        if (state === null || state.externalChange === lastExternalChange)
            return;
        const start = textarea.selectionStart;
        const end = textarea.selectionEnd;
        lastExternalChange = state.externalChange;
        tick().then(() => {
            const shift = state.cursorShift;
            const adjust = (pos: number) =>
                shift !== null && pos >= shift.from
                    ? Math.max(shift.from, pos + shift.shift)
                    : pos;
            textarea.selectionStart = adjust(start);
            textarea.selectionEnd = adjust(end);
        });
    });

    function oninput() {
        onLocalInput(textarea.value, textarea.selectionStart);
    }

    function back() {
        openNoteId.set(null);
    }
</script>

<div class="page">
    <header>
        <button class="back" onclick={back}>← Back</button>
        <h1>{note.title || "(untitled)"}</h1>
        <span class="version">
            v{$editor?.baseVersion ??
                note.version}{#if ($editor?.pending.length ?? 0) > 0}+{$editor
                    ?.pending.length}{/if}
        </span>
    </header>
    <textarea
        bind:this={textarea}
        value={$editor?.content ?? ""}
        {oninput}
        spellcheck="false"></textarea>
</div>

<style>
    .page {
        display: flex;
        flex-direction: column;
        gap: 1rem;
        height: calc(100vh - 2rem);
    }
    header {
        display: flex;
        align-items: baseline;
        gap: 1rem;
    }
    header h1 {
        flex: 1;
        margin: 0;
        font-size: 1.25rem;
    }
    .back {
        cursor: pointer;
        padding: 0.5rem 0.75rem;
    }
    .version {
        color: #888;
        font-size: 0.8rem;
    }
    textarea {
        flex: 1;
        resize: none;
        padding: 0.75rem;
        font: inherit;
        font-family: monospace;
    }
</style>
