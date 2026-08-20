<script lang="ts">
  let { audioUrl = "", transcript = "" } = $props<{
    audioUrl?: string;
    transcript?: string;
  }>();

  let isPlaying = $state(false);

  function togglePlay() {
    isPlaying = !isPlaying;
  }
</script>

<div class="bg-slate-100 rounded-lg p-3 border border-slate-200 flex flex-col gap-2">
  <div class="flex items-center gap-3">
    <button
      onclick={togglePlay}
      class="w-8 h-8 rounded-full bg-teal-600 text-white flex items-center justify-center hover:bg-teal-700 focus:outline-none"
    >
      {#if isPlaying}
        <span class="text-xs">⏸</span>
      {:else}
        <span class="text-xs">▶</span>
      {/if}
    </button>
    <div class="flex-1 bg-slate-300 h-1.5 rounded-full overflow-hidden">
      <div class="bg-teal-500 h-full w-1/3"></div>
    </div>
    <span class="text-xs text-slate-500 font-mono">0:24</span>
  </div>

  {#if transcript}
    <div class="text-xs text-slate-700 bg-white p-2 rounded border border-slate-200 italic">
      <span class="font-semibold not-italic text-slate-900">Transcript:</span> "{transcript}"
    </div>
  {/if}
</div>
