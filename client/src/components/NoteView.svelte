<script lang="ts">
    import { tick, onMount } from "svelte";
    import type { Note } from "../lib/protocol";
    import { openNoteId } from "../lib/stores";
    import { onServerEvent, onReconnect } from "../lib/ws";
    import { EditorState } from "../lib/editor";

    let { note }: { note: Note } = $props();

    let textarea: HTMLTextAreaElement;

    /* The textarea's value: owned by the component, written only on
     * external changes (typing updates the DOM directly; we never write
     * back what the user just typed). */

    /* Fixme: here, we copy the note content and handle changes at two places.
    The editor will update the opened note (and this copy of the note) and the store will also
    update all notes, changing the note content. Maybe there is a better design ? */

    // svelte-ignore state_referenced_locally
    let text = $state(note.content);
    // svelte-ignore state_referenced_locally
    let version = $state(note.version);
    // svelte-ignore state_referenced_locally
    let pending = $state(0);
    // svelte-ignore state_referenced_locally
    let resyncing = $state(false);

    let editor: EditorState | null = null;

    function refreshStatus() {
        if (editor === null) return;
        const s = editor.status();
        version = s.version;
        pending = s.pending;
        resyncing = s.resyncing;
    }

    onMount(() => {
        /* Create the editor and wire it to the socket. Everything registered
         * here is unregistered in the cleanup — one place, symmetric. */
        const e = new EditorState(note, {
            onExternalChange: (newText) => {
                const start = textarea.selectionStart;
                const end = textarea.selectionEnd;
                textarea.value = newText;
                text = newText;
                textarea.selectionStart = Math.min(start, newText.length);
                textarea.selectionEnd = Math.min(end, newText.length);
            },
            onClose: () => {
                openNoteId.set(null);
            },
        });
        editor = e;

        const unsubEvents = onServerEvent((event) => {
            e.handleEvent(event);
            refreshStatus();
        });
        const unsubReconnect = onReconnect(() => {
            e.resync(); /* Resync when the ws reconnects */
            refreshStatus();
        });

        return () => {
            unsubEvents();
            unsubReconnect();
            editor = null;
        };
    });

    function oninput() {
        editor?.userInput(textarea.value);
        refreshStatus();
    }

    function back() {
        openNoteId.set(null);
    }

    function resync() {
        if (editor) editor.resync();
    }
</script>

<div class="page">
    <header>
        <button class="back" onclick={back}> ← Back </button>
        <h1>{note.title || "(untitled)"}</h1>
        <button class="back" onclick={resync} disabled={resyncing}>
            Resync ⟳
        </button>
        <span class="version">
            v{version}{#if pending > 0}+{pending}{/if}
            {#if resyncing}⟳{/if}
        </span>
    </header>
    <textarea bind:this={textarea} value={text} {oninput} spellcheck="false"
    ></textarea>
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
        white-space: nowrap;
    }
    textarea {
        flex: 1;
        resize: none;
        padding: 0.75rem;
        font: inherit;
        font-family: monospace;
    }
</style>
