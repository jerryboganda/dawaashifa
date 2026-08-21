<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../lib/api";
  import { RxReviewManager, type PrescriptionReviewItem, type RxExtractedLine } from "../../state/rx-review";
  import { translations, type Locale, type PrescriptionDto, type QueueStatsDto } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  let manager = $state(new RxReviewManager([]));
  let queueStats = $state<QueueStatsDto | null>(null);
  let isLoading = $state(true);
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);
  let successMessage = $state<string | null>(null);

  async function loadPrescriptions() {
    isLoading = true;
    errorMessage = null;
    try {
      const [rxs, stats] = await Promise.all([
        apiFetch<PrescriptionDto[]>("/api/v1/prescriptions?status=PENDING_REVIEW"),
        apiFetch<QueueStatsDto>("/api/v1/prescriptions/queue/stats").catch(() => null),
      ]);

      queueStats = stats;

      const items: PrescriptionReviewItem[] = rxs.map((rx) => {
        const lines: RxExtractedLine[] = rx.lines.map((l) => ({
          id: l.id,
          lineNumber: l.line_no,
          rawOcrText: l.ocr_text,
          matchedProduct: {
            productId: l.matched_product_id || "",
            name: l.matched_brand_name || l.ocr_text,
            isControlled: l.is_controlled,
            confidence: l.match_confidence || 0.9,
          },
          quantity: l.qty,
          dosage: l.dosage_instructions || "As directed by physician",
          alternativeCandidates: [],
          decision: null,
        }));

        return {
          id: rx.id,
          patientName: rx.patient_name || "Patient Walk-in",
          createdAt: rx.created_at,
          imageUrl: rx.image_object_key.startsWith("http")
            ? rx.image_object_key
            : `https://images.unsplash.com/photo-1584308666744-24d5c474f2ae?w=800`,
          imageTransform: { zoom: 1, rotation: 0, contrast: false },
          lines,
        };
      });

      manager = new RxReviewManager(items);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load prescription review queue";
    } finally {
      isLoading = false;
    }
  }

  async function handleApprove() {
    if (!manager.currentItem || !manager.canApproveCurrentPrescription()) return;
    isSubmitting = true;
    errorMessage = null;
    try {
      const decisions = manager.currentItem.lines.map((l) => {
        let action: any = { type: "Accept" };
        if (l.decision === "REJECTED") {
          action = { type: "Reject", reason: "Rejected by reviewing pharmacist" };
        } else if (l.decision === "EDITED") {
          action = { type: "Edit", qty: l.quantity, dosage: l.dosage };
        } else if (l.decision === "SUBSTITUTED" && l.substitutionReason) {
          action = { type: "Substitute", product_id: l.matchedProduct.productId, reason: l.substitutionReason };
        }
        return {
          line_no: l.lineNumber,
          action,
        };
      });

      await apiFetch(`/api/v1/prescriptions/${manager.currentItem.id}/approve`, {
        method: "POST",
        body: JSON.stringify({
          decisions,
          note: "Approved via Pharmacist Ops Console",
          client_ip: "127.0.0.1",
          client_device: "Web Ops Console",
        }),
      });

      successMessage = `Prescription ${manager.currentItem.id.slice(0, 8)} approved successfully!`;
      manager.submitApproval();
      setTimeout(() => (successMessage = null), 4000);
    } catch (e: any) {
      errorMessage = e.message || "Failed to submit prescription approval";
    } finally {
      isSubmitting = false;
    }
  }

  async function handleReject() {
    if (!manager.currentItem) return;
    isSubmitting = true;
    errorMessage = null;
    try {
      await apiFetch(`/api/v1/prescriptions/${manager.currentItem.id}/reject`, {
        method: "POST",
        body: JSON.stringify({
          reason: "Prescription illegible or invalid doctor registration credentials",
        }),
      });
      successMessage = `Prescription ${manager.currentItem.id.slice(0, 8)} rejected.`;
      manager.queue.splice(manager.currentItemIndex, 1);
      setTimeout(() => (successMessage = null), 4000);
    } catch (e: any) {
      errorMessage = e.message || "Failed to reject prescription";
    } finally {
      isSubmitting = false;
    }
  }

  function handleLineAccept(lineNo: number) {
    manager.acceptLine(lineNo);
  }

  function handleLineReject(lineNo: number) {
    manager.rejectLine(lineNo);
  }

  onMount(() => {
    loadPrescriptions();
  });
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 text-slate-800">
  <!-- Top Bar: Backlog & Oldest Waiting -->
  <header class="bg-white border-b border-slate-200 px-4 py-2 flex items-center justify-between">
    <div class="flex items-center gap-4">
      <h2 class="font-bold text-sm text-slate-900">{t.rxReview.title}</h2>
      <div class="flex items-center gap-2 text-xs">
        <span class="bg-amber-100 text-amber-800 px-2 py-0.5 rounded font-medium">
          {t.rxReview.queueBacklog}: {queueStats?.total_pending ?? manager.queueDepth}
        </span>
        {#if queueStats?.oldest_waiting_seconds}
          <span class="text-slate-500 font-mono">
            {t.rxReview.oldestWaiting}: {Math.round(queueStats.oldest_waiting_seconds / 60)}m ago
          </span>
        {/if}
      </div>
    </div>

    <div class="flex items-center gap-2">
      <button
        onclick={loadPrescriptions}
        class="px-2.5 py-1 bg-white border border-slate-300 text-slate-700 text-xs font-semibold rounded hover:bg-slate-50"
      >
        🔄 Refresh
      </button>
      <span class="text-xs text-slate-500 font-mono hidden md:inline">
        Keyboard: [1-9] Line · [A] Accept · [X] Reject · [Ctrl+Enter] Approve
      </span>
    </div>
  </header>

  {#if errorMessage}
    <div class="bg-red-600 text-white text-xs px-4 py-2 font-bold flex items-center justify-between">
      <span>⚠️ {errorMessage}</span>
      <button onclick={() => (errorMessage = null)} class="text-white hover:text-red-200">✕</button>
    </div>
  {/if}

  {#if successMessage}
    <div class="bg-emerald-600 text-white text-xs px-4 py-2 font-bold flex items-center justify-between">
      <span>✅ {successMessage}</span>
      <button onclick={() => (successMessage = null)} class="text-white hover:text-emerald-200">✕</button>
    </div>
  {/if}

  {#if isLoading}
    <div class="flex-1 flex items-center justify-center text-slate-500 text-xs gap-2">
      <span class="animate-spin text-2xl">⏳</span>
      <span>Loading prescription review queue...</span>
    </div>
  {:else if !manager.currentItem}
    <div class="flex-1 flex flex-col items-center justify-center p-8 text-center text-slate-500">
      <span class="text-4xl mb-2">🎉</span>
      <h3 class="font-bold text-base text-slate-800">Prescription Queue Clean!</h3>
      <p class="text-xs text-slate-400 mt-1">No pending prescriptions requiring pharmacist review at this time.</p>
    </div>
  {:else}
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
        </div>
      </div>

      <!-- Right Pane: Extracted Lines & Decision Matrix -->
      <div class="w-[450px] bg-white border-s border-slate-200 flex flex-col">
        <div class="p-3 border-b border-slate-200 bg-slate-50 flex items-center justify-between">
          <div>
            <h3 class="font-bold text-sm text-slate-900">{manager.currentItem.patientName}</h3>
            <span class="text-[11px] font-mono text-slate-500">{manager.currentItem.createdAt}</span>
          </div>
          <span class="text-xs bg-slate-200 text-slate-700 px-2 py-0.5 rounded font-mono">
            {manager.undecidLineCount} lines pending
          </span>
        </div>

        <div class="flex-1 p-3 overflow-y-auto flex flex-col gap-3">
          {#each manager.currentItem.lines as line}
            <div
              class="border rounded-lg p-3 transition-colors flex flex-col gap-2 {line.decision === 'ACCEPTED'
                ? 'border-emerald-300 bg-emerald-50/40'
                : line.decision === 'REJECTED'
                ? 'border-red-300 bg-red-50/40'
                : 'border-slate-200 bg-white'}"
            >
              <div class="flex items-start justify-between">
                <div>
                  <span class="text-xs font-mono font-bold text-slate-400 mr-2">#{line.lineNumber}</span>
                  <span class="font-bold text-xs text-slate-900">{line.matchedProduct.name}</span>
                  {#if line.matchedProduct.isControlled}
                    <span class="ms-2 text-[10px] bg-red-100 text-red-700 px-1.5 py-0.5 rounded font-bold">
                      CONTROLLED
                    </span>
                  {/if}
                </div>
                {#if line.decision}
                  <span class="text-[11px] font-bold {line.decision === 'ACCEPTED' ? 'text-emerald-700' : 'text-red-700'}">
                    {line.decision}
                  </span>
                {/if}
              </div>

              <div class="text-xs text-slate-500 font-mono bg-slate-50 p-1.5 rounded">
                "{line.rawOcrText}"
              </div>

              <div class="flex items-center justify-between text-xs text-slate-600">
                <span>Qty: <strong>{line.quantity}</strong></span>
                <span>{line.dosage}</span>
              </div>

              <!-- Line Decision Buttons -->
              <div class="flex gap-2 pt-1 border-t border-slate-100">
                <button
                  onclick={() => handleLineAccept(line.lineNumber)}
                  class="flex-1 py-1 text-xs font-bold rounded transition-colors {line.decision === 'ACCEPTED'
                    ? 'bg-emerald-600 text-white'
                    : 'bg-slate-100 hover:bg-emerald-100 text-emerald-800'}"
                >
                  ✓ Accept
                </button>
                <button
                  onclick={() => handleLineReject(line.lineNumber)}
                  class="flex-1 py-1 text-xs font-bold rounded transition-colors {line.decision === 'REJECTED'
                    ? 'bg-red-600 text-white'
                    : 'bg-slate-100 hover:bg-red-100 text-red-800'}"
                >
                  ✕ Reject
                </button>
              </div>
            </div>
          {/each}
        </div>

        <!-- Pharmacist Approval Gate Footer -->
        <div class="p-3 border-t border-slate-200 bg-slate-50 flex flex-col gap-2">
          {#if !manager.canApproveCurrentPrescription()}
            <div class="text-[11px] text-amber-700 font-medium text-center">
              ⚠️ Invariant I-3: All lines must have an explicit decision before approval.
            </div>
          {/if}
          <div class="flex gap-2">
            <button
              onclick={handleReject}
              disabled={isSubmitting}
              class="px-4 py-2 bg-slate-200 text-slate-700 text-xs font-bold rounded hover:bg-red-100 hover:text-red-700 disabled:opacity-50"
            >
              Reject Prescription
            </button>
            <button
              onclick={handleApprove}
              disabled={!manager.canApproveCurrentPrescription() || isSubmitting}
              class="flex-1 py-2 bg-emerald-600 text-white text-xs font-bold rounded hover:bg-emerald-700 disabled:opacity-50 disabled:cursor-not-allowed shadow-sm"
            >
              {#if isSubmitting}Submitting...{:else}Approve & Advance [Ctrl+Enter]{/if}
            </button>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
