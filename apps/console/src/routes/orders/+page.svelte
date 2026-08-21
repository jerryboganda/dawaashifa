<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../lib/api";
  import { OrderBoardManager, type OrderCard, type OrderBoardStatus } from "../../state/orders";
  import MoneyDisplay from "../../components/MoneyDisplay.svelte";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import { translations, type Locale, type OrderDto } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  let manager = $state(new OrderBoardManager([]));
  let isLoading = $state(true);
  let errorMessage = $state<string | null>(null);

  const columns: { status: OrderBoardStatus; label: string }[] = [
    { status: "CONFIRMED", label: "Confirmed" },
    { status: "ALLOCATED", label: "Allocated" },
    { status: "PACKED", label: "Packed" },
    { status: "DISPATCHED", label: "Dispatched" },
    { status: "DELIVERED", label: "Delivered" },
  ];

  async function loadOrders() {
    isLoading = true;
    errorMessage = null;
    try {
      const orders = await apiFetch<OrderDto[]>("/api/v1/orders");
      const mapped: OrderCard[] = orders.map((o) => ({
        id: o.id,
        orderNumber: o.order_no,
        customerName: o.customer_id ? `Customer ${o.customer_id.slice(0, 8)}` : "Walk-in Customer",
        status: (o.status as any) || "CONFIRMED",
        totalAmount: o.total,
        itemCount: o.items ? o.items.length : 1,
        branchId: o.branch_id || "b-lhr",
        isPrescription: !!o.prescription_id,
        createdAt: o.placed_at ? new Date(o.placed_at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) : "Just now",
      }));

      manager = new OrderBoardManager(mapped);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load orders board";
    } finally {
      isLoading = false;
    }
  }

  async function handleTransition(orderId: string, nextStatus: OrderBoardStatus) {
    const res = manager.moveOrder(orderId, nextStatus);
    if (!res.success) {
      errorMessage = res.error || t.orders.illegalDropBlocked;
      setTimeout(() => (errorMessage = null), 4000);
      return;
    }

    try {
      await apiFetch(`/api/v1/orders/${orderId}/transition`, {
        method: "POST",
        body: JSON.stringify({
          target_status: nextStatus,
          reason: `Transitioned to ${nextStatus} via Ops Kanban Board`,
        }),
      });
      errorMessage = null;
    } catch (e: any) {
      errorMessage = e.message || "Backend transition rejected";
      loadOrders(); // Revert board state on failure
    }
  }

  onMount(() => {
    loadOrders();
  });
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 text-slate-800">
  <header class="bg-white border-b border-slate-200 px-4 py-2 flex items-center justify-between">
    <div class="flex items-center gap-3">
      <h2 class="font-bold text-sm text-slate-900">{t.orders.title}</h2>
      <button
        onclick={loadOrders}
        class="px-2.5 py-1 bg-white border border-slate-300 text-slate-700 text-xs font-semibold rounded hover:bg-slate-50"
      >
        🔄 Refresh
      </button>
    </div>
    <span class="text-xs bg-slate-100 text-slate-600 px-2 py-0.5 rounded font-mono">
      Active: {manager.orders.length}
    </span>
  </header>

  {#if errorMessage}
    <div class="bg-red-600 text-white text-xs px-4 py-2 font-bold flex items-center justify-between">
      <span>⚠️ {errorMessage}</span>
      <button onclick={() => (errorMessage = null)} class="text-white hover:text-red-200">✕</button>
    </div>
  {/if}

  {#if isLoading}
    <div class="flex-1 flex items-center justify-center text-slate-500 text-xs gap-2">
      <span class="animate-spin text-2xl">⏳</span>
      <span>Loading orders kanban board...</span>
    </div>
  {:else}
    <div class="flex-1 p-4 grid grid-cols-5 gap-4 overflow-x-auto">
      {#each columns as col}
        <div class="bg-slate-200/70 rounded-xl p-3 flex flex-col gap-3">
          <div class="flex items-center justify-between">
            <h3 class="font-bold text-xs uppercase tracking-wider text-slate-700">{col.label}</h3>
            <span class="text-xs bg-white px-2 py-0.5 rounded-full font-mono font-semibold text-slate-600">
              {manager.orders.filter((o) => o.status === col.status).length}
            </span>
          </div>

          <div class="flex-1 overflow-y-auto flex flex-col gap-2">
            {#each manager.orders.filter((o) => o.status === col.status) as order}
              <div class="bg-white p-3 rounded-lg shadow-sm border border-slate-200 flex flex-col gap-2">
                <div class="flex items-center justify-between">
                  <span class="font-mono text-xs font-bold text-slate-800">{order.orderNumber}</span>
                  {#if order.isPrescription}
                    <span class="text-[10px] bg-purple-100 text-purple-700 px-1 rounded font-bold">Rx</span>
                  {/if}
                </div>

                <div class="text-xs text-slate-600 flex justify-between">
                  <span>{order.customerName}</span>
                  <span class="text-slate-400 font-mono">{order.createdAt}</span>
                </div>

                <div class="flex items-center justify-between pt-1 border-t border-slate-100">
                  <span class="text-xs text-slate-500">{order.itemCount} items</span>
                  <MoneyDisplay amount={order.totalAmount} customClass="text-xs font-bold text-slate-900" />
                </div>

                <!-- Advance Action Button -->
                {#if col.status === "CONFIRMED"}
                  <button
                    onclick={() => handleTransition(order.id, "ALLOCATED")}
                    class="w-full py-1 text-[11px] font-bold bg-slate-100 hover:bg-teal-50 text-teal-800 rounded border border-slate-200"
                  >
                    Allocate →
                  </button>
                {:else if col.status === "ALLOCATED"}
                  <button
                    onclick={() => handleTransition(order.id, "PACKED")}
                    class="w-full py-1 text-[11px] font-bold bg-slate-100 hover:bg-teal-50 text-teal-800 rounded border border-slate-200"
                  >
                    Pack →
                  </button>
                {:else if col.status === "PACKED"}
                  <button
                    onclick={() => handleTransition(order.id, "DISPATCHED")}
                    class="w-full py-1 text-[11px] font-bold bg-slate-100 hover:bg-teal-50 text-teal-800 rounded border border-slate-200"
                  >
                    Dispatch →
                  </button>
                {:else if col.status === "DISPATCHED"}
                  <button
                    onclick={() => handleTransition(order.id, "DELIVERED")}
                    class="w-full py-1 text-[11px] font-bold bg-slate-100 hover:bg-emerald-50 text-emerald-800 rounded border border-slate-200"
                  >
                    Mark Delivered ✓
                  </button>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
