<script lang="ts">
  import { B2bDeskManager, type B2bAccountSummary } from "../../state/b2b";
  import MoneyDisplay from "../../components/MoneyDisplay.svelte";

  const mockAccounts: B2bAccountSummary[] = [
    {
      id: "acc-1",
      name: "Services Hospital Lahore",
      creditLimit: "5000000.0000",
      currentBalance: "3200000.0000",
      onHold: false,
      overdue90DaysMoney: "0.0000",
    },
    {
      id: "acc-2",
      name: "Doctors Hospital Ortho Clinic",
      creditLimit: "2000000.0000",
      currentBalance: "1950000.0000",
      onHold: true,
      overdue90DaysMoney: "450000.0000",
    },
  ];

  let manager = $state(new B2bDeskManager(mockAccounts));
  let recallBatch = $state("");
  let recallResults = $state<string[]>([]);

  function searchRecall() {
    if (recallBatch) {
      recallResults = [
        `Serial SN-IMP-9901: In Central Warehouse (Aisle 4)`,
        `Serial SN-IMP-9902: Consignment at Services Hospital OT 2`,
      ];
    }
  }
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 p-4 gap-4 overflow-y-auto">
  <h2 class="font-bold text-lg text-slate-900">B2B Hospital & Device Desk</h2>

  <!-- Device Recall Lookup -->
  <div class="bg-white p-4 rounded-xl border border-slate-200 shadow-sm flex flex-col gap-3">
    <h3 class="font-bold text-sm text-slate-900">Manufacturer Device Recall Query</h3>
    <div class="flex gap-2">
      <input
        type="text"
        bind:value={recallBatch}
        placeholder="Enter Lot or Batch ID..."
        class="flex-1 text-xs p-2 border border-slate-300 rounded focus:outline-none focus:ring-1 focus:ring-teal-500"
      />
      <button onclick={searchRecall} class="px-4 py-2 bg-teal-600 text-white text-xs font-bold rounded hover:bg-teal-700">
        Query Recall
      </button>
    </div>

    {#if recallResults.length > 0}
      <div class="bg-amber-50 border border-amber-200 rounded p-3 text-xs text-amber-900 flex flex-col gap-1">
        <span class="font-bold">Affected Units Found ({recallResults.length}):</span>
        {#each recallResults as r}
          <div class="font-mono">{r}</div>
        {/each}
      </div>
    {/if}
  </div>

  <!-- Accounts & Credit Summary -->
  <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
    <div class="p-4 border-b border-slate-200 font-bold text-sm text-slate-900">
      Hospital Accounts & Credit Aging
    </div>
    <table class="w-full text-start text-xs">
      <thead class="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
        <tr>
          <th class="p-3 text-start">Hospital / Account</th>
          <th class="p-3 text-start">Status</th>
          <th class="p-3 text-end">Credit Limit</th>
          <th class="p-3 text-end">Current Balance</th>
          <th class="p-3 text-end">90+ Days Overdue</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-slate-100">
        {#each manager.accounts as acc}
          <tr class="hover:bg-slate-50">
            <td class="p-3 font-semibold text-slate-900">{acc.name}</td>
            <td class="p-3">
              {#if acc.onHold}
                <span class="px-2 py-0.5 rounded font-bold bg-red-100 text-red-800">ON HOLD</span>
              {:else}
                <span class="px-2 py-0.5 rounded font-bold bg-emerald-100 text-emerald-800">ACTIVE</span>
              {/if}
            </td>
            <td class="p-3 text-end font-mono">
              <MoneyDisplay amount={acc.creditLimit} />
            </td>
            <td class="p-3 text-end font-mono">
              <MoneyDisplay amount={acc.currentBalance} />
            </td>
            <td class="p-3 text-end font-mono">
              <MoneyDisplay amount={acc.overdue90DaysMoney} customClass={acc.overdue90DaysMoney !== "0.0000" ? "text-red-600 font-bold" : ""} />
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
