<script lang="ts">
  import { RxReviewManager, type PrescriptionReviewItem } from "../../state/rx-review";
  import { translations, type Locale } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  const mockRxQueue: PrescriptionReviewItem[] = [
    {
      id: "rx-101",
      patientName: "Muhammad Usman",
      createdAt: "2026-08-20T10:15:00Z",
      imageUrl: "https://images.unsplash.com/photo-1584308666744-24d5c474f2ae?w=800",
      imageTransform: { zoom: 1, rotation: 0, contrast: false },
      lines: [
        {
          id: "l-1",
          lineNumber: 1,
          rawOcrText: "Tab Lexotanil 3mg TDS",
          matchedProduct: { productId: "prod-lexo", name: "Lexotanil 3mg", isControlled: true, confidence: 0.98 },
          quantity: 30,
          dosage: "1 tablet 3 times daily",
          alternativeCandidates: [
            { productId: "alt-1", name: "Bromazepam 3mg (Generic)", inStock: true, score: 0.92 }
          ],
          decision: null,
        },
        {
          id: "l-2",
          lineNumber: 2,
          rawOcrText: "Cap Risek 20mg OD (ac)",
          matchedProduct: { productId: "prod-risek", name: "Risek 20mg Capsule", isControlled: false, confidence: 0.96 },
          quantity: 14,
          dosage: "1 cap before breakfast",
          alternativeCandidates: [],
          decision: null,
        },
      ],
    },
  ];

  let manager = $state(new RxReviewManager(mockRxQueue));
  let isApproved = $state(false);

  function handleLineAccept(lineNo: number) {
    manager.acceptLine(lineNo);
  }

  function handleLineReject(lineNo: number) {
    manager.rejectLine(lineNo);
  }

  function handleApprove() {
    if (manager.canApproveCurrentPrescription()) {
      isApproved = manager.submitApproval();
    }
  }
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 text-slate-800">
  <!-- Top Bar: Backlog & Oldest Waiting -->
  <header class="bg-white border-b border-slate-200 px-4 py-2 flex items-center justify-between">
    <div class="flex items-center gap-4">
      <h2 class="font-bold text-sm text-slate-900">{t.rxReview.title}</h2>
      <div class="flex items-center gap-2 text-xs">
        <span class="bg-amber-100 text-amber-800 px-2 py-0.5 rounded font-medium">
          {t.rxReview.queueBacklog}: {manager.queueDepth}
        </span>
        <span class="text-slate-500 font-mono">
          {t.rxReview.oldestWaiting}: 15m ago
        </span>
      </div>
    </div>

    <div>
      <span class="text-xs text-slate-500 font-mono">
        Keyboard: [1-9] Line · [A] Accept · [X] Reject · [Ctrl+Enter] Approve
      </span>
    </div>
  </header>

  {#if manager.currentItem}
    {#if manager.hasControlledSubstance}
      <div class="bg-red-600 text-white text-xs px-4 py-2 font-bold flex items-center gap-2">
        <span>⚠️</span>
        <span>{t.rxReview.controlledWarning}</span>
      </div>
    {/if}

    <div class="flex-1 flex overflow-hidden">
      <!-- Left Pane: Prescription Image -->
      <div class="flex-1 bg-slate-900 flex flex-col items-center justify-center p-4 relative">
        <img
          src={manager.currentItem.imageUrl}
          alt="Prescription"
          class="max-h-full max-w-full object-contain rounded shadow-lg border border-slate-700"
        />
        <div class="absolute bottom-4 left-4 bg-slate-800/90 text-white text-xs px-3 py-1.5 rounded-lg flex gap-3">
          <button class="hover:text-teal-400">🔍 Zoom In</button>
          <button class="hover:text-teal-400">🔄 Rotate</button>
          <button class="hover:text-teal-400">🌓 Contrast</button>
        </div>
      </div>

      <!-- Right Pane: Extracted Lines -->
      <div class="w-[500px] bg-white border-s border-slate-200 flex flex-col">
        <div class="p-3 border-b border-slate-200 flex items-center justify-between">
          <h3 class="font-bold text-sm text-slate-900">Extracted Medications</h3>
          <span class="text-xs font-semibold text-teal-700 bg-teal-50 px-2 py-0.5 rounded">
            {manager.undecidLineCount} {t.rxReview.linesRemaining}
          </span>
        </div>

        <div class="flex-1 overflow-y-auto p-3 flex flex-col gap-3">
          {#each manager.currentItem.lines as line}
            <div class="border rounded-lg p-3 transition-colors {line.decision === 'ACCEPTED' ? 'border-emerald-300 bg-emerald-50/40' : line.decision === 'REJECTED' ? 'border-red-300 bg-red-50/40' : 'border-slate-200 bg-white'}">
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs font-bold text-slate-500">#{line.lineNumber}</span>
                <span class="text-xs font-semibold px-2 py-0.5 rounded {line.matchedProduct.confidence > 0.9 ? 'bg-teal-100 text-teal-800' : 'bg-amber-100 text-amber-800'}">
                  {t.rxReview.confidence}: {(line.matchedProduct.confidence * 100).toFixed(0)}%
                </span>
              </div>

              <div class="text-xs text-slate-500 mb-1">
                <span class="font-semibold">{t.rxReview.ocrRaw}:</span> <code class="bg-slate-100 px-1 py-0.5 rounded">{line.rawOcrText}</code>
              </div>

              <div class="text-sm font-bold text-slate-900 mb-2">
                {line.matchedProduct.name}
              </div>

              <div class="flex items-center gap-2 mb-3">
                <span class="text-xs text-slate-600">Qty: {line.quantity}</span>
                <span class="text-xs text-slate-400">·</span>
                <span class="text-xs text-slate-600">Dosage: {line.dosage}</span>
              </div>

              <div class="flex items-center justify-between pt-2 border-t border-slate-100">
                <div class="flex gap-2">
                  <button
                    onclick={() => handleLineAccept(line.lineNumber)}
                    class="px-2.5 py-1 text-xs font-semibold rounded {line.decision === 'ACCEPTED' ? 'bg-emerald-600 text-white' : 'bg-slate-100 text-slate-700 hover:bg-emerald-100'}"
                  >
                    {t.common.accept}
                  </button>
                  <button
                    onclick={() => handleLineReject(line.lineNumber)}
                    class="px-2.5 py-1 text-xs font-semibold rounded {line.decision === 'REJECTED' ? 'bg-red-600 text-white' : 'bg-slate-100 text-slate-700 hover:bg-red-100'}"
                  >
                    {t.common.reject}
                  </button>
                </div>

                {#if line.decision}
                  <span class="text-xs font-bold uppercase tracking-wider {line.decision === 'ACCEPTED' ? 'text-emerald-700' : 'text-red-700'}">
                    ✓ {line.decision}
                  </span>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        <!-- Approval Footer (Doc 16 §7: strictly disabled until all lines decided) -->
        <div class="p-4 border-t border-slate-200 bg-slate-50 flex flex-col gap-2">
          {#if !manager.canApproveCurrentPrescription()}
            <p class="text-[11px] text-amber-700 font-medium">
              ⓘ {t.rxReview.allLinesRequired}
            </p>
          {/if}

          <button
            disabled={!manager.canApproveCurrentPrescription()}
            onclick={handleApprove}
            class="w-full py-2 px-4 rounded font-bold text-sm transition-colors {manager.canApproveCurrentPrescription() ? 'bg-emerald-600 text-white hover:bg-emerald-700' : 'bg-slate-300 text-slate-500 cursor-not-allowed'}"
          >
            {t.rxReview.approvePrescription}
          </button>
        </div>
      </div>
    </div>
  {:else}
    <div class="flex-1 flex items-center justify-center text-slate-400">
      {t.common.empty}
    </div>
  {/if}
</div>
