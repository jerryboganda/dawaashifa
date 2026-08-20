<script lang="ts">
  import { OrderBoardManager, type OrderCard, type OrderBoardStatus } from "../../state/orders";
  import MoneyDisplay from "../../components/MoneyDisplay.svelte";
  import StatusBadge from "../../components/StatusBadge.svelte";
  import { translations, type Locale } from "@shifa/shared";

  let { currentLocale = "en" as Locale } = $props<{ currentLocale?: Locale }>();
  let t = $derived(translations[currentLocale]);

  const mockOrders: OrderCard[] = [
    {
      id: "ord-1",
      orderNumber: "ORD-2026-001",
      customerName: "Dr. Asim Shah",
      status: "CONFIRMED",
      totalAmount: "4500.0000",
      itemCount: 4,
      branchId: "b-lhr",
      isPrescription: true,
      createdAt: "10:15 AM",
    },
    {
      id: "ord-2",
      orderNumber: "ORD-2026-002",
      customerName: "Noreen Fatima",
      status: "ALLOCATED",
      totalAmount: "1200.0000",
      itemCount: 2,
      branchId: "b-lhr",
      isPrescription: false,
      createdAt: "11:00 AM",
    },
    {
      id: "ord-3",
      orderNumber: "ORD-2026-003",
      customerName: "Rashid Minhas",
      status: "DISPATCHED",
      totalAmount: "890.0000",
      itemCount: 1,
      branchId: "b-lhr",
      isPrescription: false,
      createdAt: "11:45 AM",
    },
  ];

  let manager = $state(new OrderBoardManager(mockOrders));
  let errorMessage = $state<string | null>(null);

  const columns: { status: OrderBoardStatus; label: string }[] = [
    { status: "CONFIRMED", label: "Confirmed" },
    { status: "ALLOCATED", label: "Allocated" },
    { status: "PACKED", label: "Packed" },
    { status: "DISPATCHED", label: "Dispatched" },
    { status: "DELIVERED", label: "Delivered" },
  ];

  function handleTransition(orderId: string, nextStatus: OrderBoardStatus) {
    const res = manager.moveOrder(orderId, nextStatus);
    if (!res.success) {
      errorMessage = res.error || t.orders.illegalDropBlocked;
      setTimeout(() => (errorMessage = null), 4000);
    } else {
      errorMessage = null;
    }
  }
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 text-slate-800">
  <header class="bg-white border-b border-slate-200 px-4 py-2 flex items-center justify-between">
    <h2 class="font-bold text-sm text-slate-900">{t.orders.title}</h2>
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

              <div class="text-xs font-semibold text-slate-900">{order.customerName}</div>

              <div class="flex items-center justify-between text-xs text-slate-500 pt-2 border-t border-slate-100">
                <span>{order.itemCount} items</span>
                <MoneyDisplay amount={order.totalAmount} customClass="font-bold text-slate-900" />
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>
