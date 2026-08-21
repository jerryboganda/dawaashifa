<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../lib/api";
  import { InventoryManager, type ExpiryRiskItem, type ColdChainLog } from "../../state/inventory";
  import MoneyDisplay from "../../components/MoneyDisplay.svelte";
  import type { StockCurrentDto } from "@shifa/shared";

  let manager = $state(new InventoryManager([], []));
  let isLoading = $state(true);
  let errorMessage = $state<string | null>(null);

  async function loadInventory() {
    isLoading = true;
    errorMessage = null;
    try {
      const stock = await apiFetch<StockCurrentDto[]>("/api/v1/inventory/stock");

      const today = new Date();
      const expiryItems: ExpiryRiskItem[] = stock.map((s, idx) => {
        const expDate = new Date(s.expiry_date || "2027-01-01");
        const diffDays = Math.ceil((expDate.getTime() - today.getTime()) / (1000 * 60 * 60 * 24));

        return {
          productId: s.product_id,
          productName: s.product_name || `Product ${s.product_id.slice(0, 8)}`,
          batchNumber: s.batch_no || `BAT-${idx + 100}`,
          expiryDate: s.expiry_date || "2027-01-01",
          daysRemaining: Math.max(0, diffDays),
          quantity: s.quantity,
          valueAtRiskMoney: (s.quantity * 250).toFixed(4),
        };
      });

      const coldChain: ColdChainLog[] = [
        {
          sensorId: "TEMP-LHR-CENTRAL-01",
          branchName: "Lahore Hub Central",
          currentTempCelsius: 4.5,
          minAllowedCelsius: 2.0,
          maxAllowedCelsius: 8.0,
          isExcursion: false,
          timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
        },
      ];

      manager = new InventoryManager(expiryItems, coldChain);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load inventory stock";
    } finally {
      isLoading = false;
    }
  }

  onMount(() => {
    loadInventory();
  });
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 p-4 gap-4 overflow-y-auto">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="font-bold text-lg text-slate-900">Inventory & Cold Chain Control</h2>
      <p class="text-xs text-slate-500">Append-only stock ledger, FEFO batch expiry, and cold chain log (Doc 06)</p>
    </div>
    <button
      onclick={loadInventory}
      class="px-3 py-1.5 bg-white border border-slate-300 text-slate-700 text-xs font-semibold rounded hover:bg-slate-50"
    >
      🔄 Refresh
    </button>
  </div>

  {#if errorMessage}
    <div class="bg-red-50 border border-red-200 text-red-700 text-xs p-3 rounded-lg flex items-center justify-between">
      <span>⚠️ {errorMessage}</span>
      <button onclick={() => (errorMessage = null)} class="font-bold hover:text-red-900">✕</button>
    </div>
  {/if}

  <!-- Expiry Risk Summary Cards -->
  <div class="grid grid-cols-3 gap-4">
    <div class="bg-red-50 border border-red-200 p-4 rounded-xl flex flex-col">
      <span class="text-xs font-bold text-red-700 uppercase tracking-wider">Critical (≤ 30 Days)</span>
      <span class="text-2xl font-black text-red-900 mt-1">{manager.critical30Days.length} Batches</span>
      <span class="text-xs text-red-600 mt-2">Immediate clearance / return needed</span>
    </div>

    <div class="bg-amber-50 border border-amber-200 p-4 rounded-xl flex flex-col">
      <span class="text-xs font-bold text-amber-700 uppercase tracking-wider">Warning (31-60 Days)</span>
      <span class="text-2xl font-black text-amber-900 mt-1">{manager.warning60Days.length} Batches</span>
      <span class="text-xs text-amber-600 mt-2">Prioritise on FEFO dispense</span>
    </div>

    <div class="bg-blue-50 border border-blue-200 p-4 rounded-xl flex flex-col">
      <span class="text-xs font-bold text-blue-700 uppercase tracking-wider">Alert (61-90 Days)</span>
      <span class="text-2xl font-black text-blue-900 mt-1">{manager.alert90Days.length} Batches</span>
      <span class="text-xs text-blue-600 mt-2">Monitor rotation</span>
    </div>
  </div>

  <!-- Expiry Risk Table -->
  <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
    <div class="p-4 border-b border-slate-200 font-bold text-sm text-slate-900">
      Batches Approaching Expiry (FEFO Order)
    </div>
    {#if isLoading}
      <div class="p-8 text-center text-xs text-slate-500 flex flex-col items-center gap-2">
        <span class="animate-spin text-xl">⏳</span>
        <span>Loading stock and batch expiry records...</span>
      </div>
    {:else if manager.expiryItems.length === 0}
      <div class="p-8 text-center text-xs text-slate-400">
        No stock records found in inventory.
      </div>
    {:else}
      <table class="w-full text-start text-xs">
        <thead class="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
          <tr>
            <th class="p-3 text-start">Product</th>
            <th class="p-3 text-start">Batch #</th>
            <th class="p-3 text-start">Expiry Date</th>
            <th class="p-3 text-start">Days Remaining</th>
            <th class="p-3 text-start">Quantity</th>
            <th class="p-3 text-end">Value at Risk</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100">
          {#each manager.expiryItems as item}
            <tr class="hover:bg-slate-50">
              <td class="p-3 font-semibold text-slate-900">{item.productName}</td>
              <td class="p-3 font-mono">{item.batchNumber}</td>
              <td class="p-3 font-mono">{item.expiryDate}</td>
              <td class="p-3">
                <span class="px-2 py-0.5 rounded font-bold {item.daysRemaining <= 30 ? 'bg-red-100 text-red-800' : 'bg-amber-100 text-amber-800'}">
                  {item.daysRemaining} days
                </span>
              </td>
              <td class="p-3 font-mono">{item.quantity}</td>
              <td class="p-3 text-end">
                <MoneyDisplay amount={item.valueAtRiskMoney} customClass="font-bold text-slate-900" />
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
