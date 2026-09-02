<script lang="ts">
    import type { Note } from "../lib/protocol";
    import { deleteNoteRequest } from "../lib/api";

    let { note, onclose }: { note: Note; onclose: () => void } = $props();

    let submitting = $state(false);
    let error = $state<string | null>(null);

    async function remove() {
        if (submitting) return;
        submitting = true;
        error = null;
        try {
            const response = await deleteNoteRequest(note.id);
            if (response.result === "Ok") {
                onclose();
            } else {
                error = response.message;
            }
        } catch (e) {
            error = e instanceof Error ? e.message : "request failed";
        } finally {
            submitting = false;
        }
    }
</script>

<div
    class="backdrop"
    role="presentation"
    onclick={(e) => {
        if (e.target === e.currentTarget) onclose();
    }}
>
    <div
        class="dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Delete note"
    >
        <h2>Delete note</h2>
        <p>
            Delete <strong>{note.title || "(untitled)"}</strong>? This cannot be
            undone.
        </p>
        {#if error !== null}
            <p class="error">{error}</p>
        {/if}
        <div class="actions">
            <button onclick={onclose}>Cancel</button>
            <button class="danger" disabled={submitting} onclick={remove}>
                {submitting ? "Deleting…" : "Delete"}
            </button>
        </div>
    </div>
</div>

<style>
    .backdrop {
        position: fixed;
        inset: 0;
        background: rgba(0, 0, 0, 0.4);
        display: flex;
        align-items: center;
        justify-content: center;
        z-index: 10;
    }
    .dialog {
        background: white;
        border-radius: 6px;
        padding: 1.25rem;
        min-width: 20rem;
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.25);
    }
    .dialog h2 {
        margin: 0;
        font-size: 1.1rem;
    }
    .dialog p {
        margin: 0;
    }
    .error {
        margin: 0;
        color: #c0392b;
        font-size: 0.85rem;
    }
    .actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
    }
    .actions button {
        padding: 0.5rem 0.9rem;
        cursor: pointer;
    }
    .danger {
        background: #c0392b;
        color: white;
        border-color: #c0392b;
    }
    .danger:disabled {
        opacity: 0.5;
        cursor: default;
    }
</style>
