<script lang="ts">
    import NoteList from "./components/NoteList.svelte";
    import NoteView from "./components/NoteView.svelte";
    import { openNote } from "./lib/stores";
</script>

<main>
    {#if $openNote !== null}
        <NoteView note={$openNote} />
    {:else}
        <NoteList />
    {/if}
</main>

<style>
    main {
        max-width: 640px;
        margin: 0 auto;
        padding: 1rem;
    }
</style>
