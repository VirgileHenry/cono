<script lang="ts">
    import { createNoteRequest } from "../lib/api";

    let { onclose }: { onclose: () => void } = $props();

    let title = $state("");
    let submitting = $state(false);
    let error = $state<string | null>(null);

    /* Disallow empty and whitespace-only names. */
    let valid = $derived(title.trim().length > 0);

    async function create() {
        if (!valid || submitting) return;
        submitting = true;
        error = null;
        try {
            const response = await createNoteRequest(title.trim());
            if (response.result == "Ok") {
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
        if (event.key == "Enter") create();
        if (event.key == "Escape") onclose();
    }
</script>

<div
    class="backdrop"
    role="presentation"
    onclick={(e) => {
        if (e.target === e.currentTarget) onclose();
    }}
>
    <div class="dialog" role="dialog" aria-modal="true" aria-label="New note">
        <h2>New note</h2>
        <!-- svelte-ignore a11y_autofocus -->
        <input
            type="text"
            placeholder="Note title"
            bind:value={title}
            {onkeydown}
            autofocus
        />
        {#if error !== null}
            <p class="error">{error}</p>
        {/if}
        <div class="actions">
            <button onclick={onclose}>Cancel</button>
            <button
                class="primary"
                disabled={!valid || submitting}
                onclick={create}
            >
                {submitting ? "Creating…" : "Create"}
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
