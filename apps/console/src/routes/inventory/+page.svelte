<script lang="ts">
  import { InventoryManager, type ExpiryRiskItem, type ColdChainLog } from "../../state/inventory";
  import MoneyDisplay from "../../components/MoneyDisplay.svelte";

  const mockExpiry: ExpiryRiskItem[] = [
    {
      productId: "p-1",
      productName: "Insulin Glargine 100IU/ml",
      batchNumber: "BAT-9921",
      expiryDate: "2026-09-15",
      daysRemaining: 25,
      quantity: 80,
      valueAtRiskMoney: "240000.0000",
    },
    {
      productId: "p-2",
      productName: "Ceftriaxone 1g Injection",
      batchNumber: "BAT-8812",
      expiryDate: "2026-10-20",
      daysRemaining: 58,
      quantity: 150,
      valueAtRiskMoney: "75000.0000",
    },
  ];

  const mockColdChain: ColdChainLog[] = [
    {
      sensorId: "TEMP-LHR-FRIDGE-01",
      branchName: "Lahore Model Town",
      currentTempCelsius: 4.2,
      minAllowedCelsius: 2.0,
      maxAllowedCelsius: 8.0,
      isExcursion: false,
      timestamp: "11:55 AM",
    },
  ];

  let manager = $state(new InventoryManager(mockExpiry, mockColdChain));
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 p-4 gap-4 overflow-y-auto">
  <div class="flex items-center justify-between">
    <h2 class="font-bold text-lg text-slate-900">Inventory & Cold Chain Control</h2>
  </div>

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
      Batches Approaching Expiry
    </div>
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
  </div>
</div>
