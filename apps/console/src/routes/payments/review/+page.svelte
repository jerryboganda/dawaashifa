<script lang="ts">
  import { PaymentReviewManager, type PaymentReviewItem } from "../../../state/payments";
  import MoneyDisplay from "../../../components/MoneyDisplay.svelte";
  import { translations, type Locale } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  const mockPaymentQueue: PaymentReviewItem[] = [
    {
      id: "pay-101",
      orderId: "ORD-2026-8801",
      customerName: "Imran Ali",
      orderTotalMoney: "3200.0000",
      ocrExtractedMoney: "3200.0000",
      screenshotUrl: "https://images.unsplash.com/photo-1554224155-8d04cb21cd6c?w=800",
      transactionId: "EP-982341",
      flags: [
        {
          code: "DUPLICATE_TID",
          severity: "CRITICAL",
          description: "Transaction ID already used on Order #ORD-2026-7712",
          earlierOrderId: "ORD-2026-7712",
        },
      ],
      decision: null,
    },
  ];

  let manager = $state(new PaymentReviewManager(mockPaymentQueue));

  function handleApprove(id: string) {
    manager.approvePayment(id);
  }

  function handleReject(id: string) {
    manager.rejectPayment(id, "Duplicate TID detected");
  }
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 text-slate-800">
  <header class="bg-white border-b border-slate-200 px-4 py-2 flex items-center justify-between">
    <h2 class="font-bold text-sm text-slate-900">{t.payments.title}</h2>
    <span class="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-mono">
      Queue: {manager.queue.length}
    </span>
  </header>

  {#if manager.currentItem}
    <!-- Critical Duplicate TID Banner (Doc 16 §8) -->
    {#if manager.duplicateTidFlag}
      <div class="bg-red-600 text-white px-4 py-3 font-bold text-sm flex items-center justify-between shadow-md">
        <div class="flex items-center gap-2">
          <span class="text-lg">🚨</span>
          <span>{t.payments.duplicateTidWarning} (Ref: {manager.duplicateTidFlag.earlierOrderId})</span>
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

        <!-- Decision Actions -->
        <div class="mt-auto flex gap-3 pt-4 border-t border-slate-200">
          <button
            onclick={() => handleReject(manager.currentItem!.id)}
            class="flex-1 py-2 bg-red-600 text-white font-bold text-sm rounded hover:bg-red-700 transition-colors"
          >
            {t.payments.rejectPayment}
          </button>
          <button
            onclick={() => handleApprove(manager.currentItem!.id)}
            class="flex-1 py-2 bg-emerald-600 text-white font-bold text-sm rounded hover:bg-emerald-700 transition-colors"
          >
            {t.payments.approvePayment}
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
