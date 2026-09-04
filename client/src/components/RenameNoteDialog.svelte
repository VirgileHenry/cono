<script lang="ts">
    import type { Note } from "../lib/protocol";
    import { renameNoteRequest } from "../lib/api";

    let { note, onclose }: { note: Note; onclose: () => void } = $props();

    /* Prefill with the current title, deliberate snapshot. */
    // svelte-ignore state_referenced_locally
    let title = $state(note.title);
    let submitting = $state(false);
    let error = $state<string | null>(null);

    let valid = $derived(title.trim().length > 0);

    async function rename() {
        if (!valid || submitting) return;
        submitting = true;
        error = null;
        try {
            const response = await renameNoteRequest(
                note.id,
                note.version,
                title.trim(),
            );
            if (response.result === "ok") {
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

    function onkeydown(event: KeyboardEvent) {
        if (event.key === "Enter") rename();
        if (event.key === "Escape") onclose();
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
        aria-label="Rename note"
    >
        <h2>Rename note</h2>
        <!-- svelte-ignore a11y_autofocus -->
        <input type="text" bind:value={title} {onkeydown} autofocus />
        {#if error !== null}
            <p class="error">{error}</p>
        {/if}
        <div class="actions">
            <button onclick={onclose}>Cancel</button>
            <button
                class="primary"
                disabled={!valid || submitting}
                onclick={rename}
            >
                {submitting ? "Renaming…" : "Rename"}
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
    input {
        padding: 0.5rem 0.65rem;
        font: inherit;
        border: 1px solid #ccc;
        border-radius: 4px;
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
    .primary {
        background: #2c6fdb;
        color: white;
        border-color: #2c6fdb;
    }
    .primary:disabled {
        opacity: 0.5;
        cursor: default;
    }
</style>
