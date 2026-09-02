<script lang="ts">
    import type { Note } from "../lib/protocol";

    let {
        note,
        onopen,
        onrename,
        ondelete,
    }: {
        note: Note;
        onopen: () => void;
        onrename: () => void;
        ondelete: () => void;
    } = $props();

    let menuOpen = $state(false);

    function toggleMenu(event: MouseEvent) {
        event.stopPropagation();
        menuOpen = !menuOpen;
    }

    function renameNote(event: MouseEvent) {
        event.stopPropagation();
        menuOpen = false;
        onrename();
    }

    function deleteNote(event: MouseEvent) {
        event.stopPropagation();
        menuOpen = false;
        ondelete();
    }
</script>

<li class="entry">
    <button class="open" onclick={onopen}>
        <span class="title">{note.title || "(untitled)"}</span>
        <span class="date">{new Date(note.updated_at).toLocaleString()}</span>
    </button>
    <div class="menu-anchor">
        <button class="dots" onclick={toggleMenu}>⋮</button>
        {#if menuOpen}
            <div class="menu">
                <button onclick={renameNote}>Rename</button>
                <button class="danger" onclick={deleteNote}>Delete</button>
            </div>
        {/if}
    </div>
</li>

<style>
    .entry {
        display: flex;
        align-items: stretch;
        gap: 0.25rem;
    }
    .open {
        flex: 1;
        display: flex;
        justify-content: space-between;
        align-items: baseline;
        gap: 1rem;
        padding: 0.75rem;
        cursor: pointer;
        text-align: left;
    }
    .title {
        font-weight: 600;
    }
    .date {
        font-size: 0.8rem;
        color: #888;
        white-space: nowrap;
    }
    .menu-anchor {
        position: relative;
        display: flex;
    }
    .dots {
        padding: 0 0.75rem;
        cursor: pointer;
        font-size: 1.1rem;
    }
    .menu {
        position: absolute;
        right: 0;
        top: 100%;
        z-index: 1;
        display: flex;
        flex-direction: column;
        min-width: 8rem;
        background: white;
        border: 1px solid #ccc;
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
    }
    .menu button {
        padding: 0.5rem 0.75rem;
        text-align: left;
        cursor: pointer;
        border: none;
        background: none;
    }
    .menu button:hover {
        background: #f0f0f0;
    }
    .danger {
        color: #c0392b;
    }
</style>
