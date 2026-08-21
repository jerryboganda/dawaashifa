<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../../lib/api";
  import { PaymentReviewManager, type PaymentReviewItem } from "../../../state/payments";
  import MoneyDisplay from "../../../components/MoneyDisplay.svelte";
  import { translations, type Locale, type PaymentProofDto } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  let manager = $state(new PaymentReviewManager([]));
  let isLoading = $state(true);
  let isSubmitting = $state(false);
  let errorMessage = $state<string | null>(null);
  let successMessage = $state<string | null>(null);

  async function loadProofQueue() {
    isLoading = true;
    errorMessage = null;
    try {
      const proofs = await apiFetch<PaymentProofDto[]>("/api/v1/payments/proofs/queue");
      const mapped: PaymentReviewItem[] = proofs.map((p) => ({
        id: p.id,
        orderId: p.order_id,
        customerName: p.ocr_sender || "Customer Transfer",
        orderTotalMoney: p.ocr_amount || "0.0000",
        ocrExtractedMoney: p.ocr_amount || "0.0000",
        screenshotUrl: p.image_object_key.startsWith("http")
          ? p.image_object_key
          : "https://images.unsplash.com/photo-1554224155-8d04cb21cd6c?w=800",
        transactionId: p.ocr_tid || "TID-UNKNOWN",
        flags: (p.fraud_flags || []).map((f: any) => ({
          code: f.code || "FLAG",
          severity: f.severity || "MEDIUM",
          description: f.description || f.code,
          earlierOrderId: f.earlier_order_id,
        })),
        decision: null,
      }));

      manager = new PaymentReviewManager(mapped);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load payment review queue";
    } finally {
      isLoading = false;
    }
  }

  async function handleApprove(id: string) {
    isSubmitting = true;
    errorMessage = null;
    try {
      await apiFetch(`/api/v1/payments/proofs/${id}/approve`, {
        method: "POST",
        body: JSON.stringify({
          review_note: "Approved by finance operator in console",
        }),
      });
      manager.approvePayment(id);
      successMessage = `Payment proof ${id.slice(0, 8)} approved and order marked CONFIRMED!`;
      setTimeout(() => (successMessage = null), 4000);
    } catch (e: any) {
      errorMessage = e.message || "Failed to approve payment proof";
    } finally {
      isSubmitting = false;
    }
  }

  async function handleReject(id: string) {
    isSubmitting = true;
    errorMessage = null;
    try {
      await apiFetch(`/api/v1/payments/proofs/${id}/reject`, {
        method: "POST",
        body: JSON.stringify({
          reason: "Payment proof rejected due to invalid transaction ID / amount mismatch",
        }),
      });
      manager.rejectPayment(id, "Rejected by finance desk");
      successMessage = `Payment proof ${id.slice(0, 8)} rejected.`;
      setTimeout(() => (successMessage = null), 4000);
    } catch (e: any) {
      errorMessage = e.message || "Failed to reject payment proof";
    } finally {
      isSubmitting = false;
    }
  }

  onMount(() => {
    loadProofQueue();
  });
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 text-slate-800">
  <header class="bg-white border-b border-slate-200 px-4 py-2 flex items-center justify-between">
    <h2 class="font-bold text-sm text-slate-900">{t.payments.title}</h2>
    <div class="flex items-center gap-2">
      <button
        onclick={loadProofQueue}
        class="px-2.5 py-1 bg-white border border-slate-300 text-slate-700 text-xs font-semibold rounded hover:bg-slate-50"
      >
        🔄 Refresh
      </button>
      <span class="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-mono">
        Queue: {manager.queue.length}
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
      <span>Loading payment review queue...</span>
    </div>
  {:else if !manager.currentItem}
    <div class="flex-1 flex flex-col items-center justify-center p-8 text-center text-slate-500">
      <span class="text-4xl mb-2">🎉</span>
      <h3 class="font-bold text-base text-slate-800">Payment Review Queue Clean!</h3>
      <p class="text-xs text-slate-400 mt-1">No screenshot payment proofs awaiting human review.</p>
    </div>
  {:else}
    <!-- Critical Duplicate TID Banner (Doc 16 §8, Invariant I-4) -->
    {#if manager.duplicateTidFlag}
      <div class="bg-red-600 text-white px-4 py-3 font-bold text-sm flex items-center justify-between shadow-md">
        <div class="flex items-center gap-2">
          <span class="text-lg">🚨</span>
          <span>{t.payments.duplicateTidWarning} (Ref: {manager.duplicateTidFlag.earlierOrderId || "Previous Order"})</span>
        </div>
        <span class="text-xs bg-red-800 px-2 py-1 rounded font-mono">CRITICAL SEVERITY</span>
      </div>
    {/if}

    <div class="flex-1 flex overflow-hidden">
      <!-- Pane 1: Proof Screenshot -->
      <div class="flex-1 bg-slate-900 p-4 flex items-center justify-center">
        <img
          src={manager.currentItem.screenshotUrl}
          alt="Payment Receipt"
          class="max-h-full max-w-full object-contain rounded shadow border border-slate-700"
        />
      </div>

      <!-- Pane 2 & 3: Fraud Flags & Order Match -->
      <div class="w-[450px] bg-white border-s border-slate-200 flex flex-col p-4">
        <h3 class="font-bold text-sm text-slate-900 mb-4">Payment Verification</h3>

        <div class="bg-slate-50 border border-slate-200 rounded-lg p-3 mb-4 flex flex-col gap-2">
          <div class="flex justify-between text-xs">
            <span class="text-slate-500">Order ID:</span>
            <span class="font-mono font-semibold">{manager.currentItem.orderId}</span>
          </div>
          <div class="flex justify-between text-xs">
            <span class="text-slate-500">Customer:</span>
            <span class="font-semibold">{manager.currentItem.customerName}</span>
          </div>
          <div class="flex justify-between text-xs">
            <span class="text-slate-500">Transaction ID:</span>
            <span class="font-mono font-bold text-teal-700">{manager.currentItem.transactionId}</span>
          </div>
        </div>

        <!-- Side-by-Side Amounts -->
        <div class="grid grid-cols-2 gap-3 mb-4">
          <div class="p-3 rounded-lg border border-slate-200 bg-slate-50">
            <span class="text-[11px] text-slate-500 block">Order Total</span>
            <MoneyDisplay amount={manager.currentItem.orderTotalMoney} customClass="text-sm font-bold text-slate-900" />
          </div>
          <div class="p-3 rounded-lg border {manager.hasAmountMismatch ? 'border-red-300 bg-red-50' : 'border-slate-200 bg-slate-50'}">
            <span class="text-[11px] text-slate-500 block">Proof OCR Amount</span>
            <MoneyDisplay amount={manager.currentItem.ocrExtractedMoney} customClass="text-sm font-bold text-slate-900" />
          </div>
        </div>

        <!-- Risk Flags -->
        <div class="flex-1 overflow-y-auto mb-4">
          <h4 class="font-bold text-xs text-slate-700 uppercase mb-2">Automated Risk Flags ({manager.currentItem.flags.length})</h4>
          {#if manager.currentItem.flags.length === 0}
            <div class="text-xs text-emerald-600 bg-emerald-50 p-2 rounded">
              ✓ No duplicate TID or timestamp anomalies detected.
            </div>
          {:else}
            <div class="flex flex-col gap-2">
              {#each manager.currentItem.flags as flag}
                <div class="p-2.5 rounded border text-xs flex flex-col gap-1 {flag.severity === 'CRITICAL' ? 'bg-red-50 border-red-200 text-red-900' : 'bg-amber-50 border-amber-200 text-amber-900'}">
                  <div class="flex items-center justify-between font-bold">
                    <span>{flag.code}</span>
                    <span class="text-[10px] uppercase font-mono">{flag.severity}</span>
                  </div>
                  <p class="text-[11px]">{flag.description}</p>
                </div>
              {/each}
            </div>
          {/if}
        </div>

        <!-- Review Actions (Invariant I-4) -->
        <div class="flex gap-2 pt-2 border-t border-slate-200">
          <button
            onclick={() => handleReject(manager.currentItem!.id)}
            disabled={isSubmitting}
            class="flex-1 py-2 bg-red-600 text-white text-xs font-bold rounded hover:bg-red-700 disabled:opacity-50"
          >
            {t.payments.reject}
          </button>
          <button
            onclick={() => handleApprove(manager.currentItem!.id)}
            disabled={isSubmitting}
            class="flex-1 py-2 bg-emerald-600 text-white text-xs font-bold rounded hover:bg-emerald-700 disabled:opacity-50 shadow-sm"
          >
            {t.payments.approve}
          </button>
        </div>
      </div>
    </div>
  {/if}
</div>
