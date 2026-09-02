<script lang="ts">
    import type { Note } from "../lib/protocol";
    import { sortedNotes, openNoteId } from "../lib/stores";
    import NoteListEntry from "./NoteListEntry.svelte";
    import CreateNoteDialog from "./CreateNoteDialog.svelte";
    import RenameNoteDialog from "./RenameNoteDialog.svelte";
    import DeleteNoteDialog from "./DeleteNoteDialog.svelte";

    let creating = $state(false);
    let renaming = $state<Note | null>(null);
    let deleting = $state<Note | null>(null);

    function openNote(id: string) {
        openNoteId.set(id);
    }
</script>

<div class="page">
    <h1>Notes</h1>
    <ul class="note-list">
        {#each $sortedNotes as note (note.id)}
            <NoteListEntry
                {note}
                onopen={() => openNote(note.id)}
                onrename={() => (renaming = note)}
                ondelete={() => (deleting = note)}
            />
        {:else}
            <li class="empty">No notes yet.</li>
        {/each}
    </ul>
    <button class="new-note" onclick={() => (creating = true)}
        >+ New note</button
    >
</div>

{#if creating}
    <CreateNoteDialog onclose={() => (creating = false)} />
{/if}
{#if renaming !== null}
    <RenameNoteDialog note={renaming} onclose={() => (renaming = null)} />
{/if}
{#if deleting !== null}
    <DeleteNoteDialog note={deleting} onclose={() => (deleting = null)} />
{/if}

<style>
    .page {
        display: flex;
        flex-direction: column;
        gap: 1rem;
    }
    .note-list {
        list-style: none;
        margin: 0;
        padding: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
    .empty {
        color: #888;
        padding: 1rem;
        text-align: center;
    }
    .new-note {
        padding: 0.75rem;
        font-size: 1rem;
        cursor: pointer;
    }
</style>
