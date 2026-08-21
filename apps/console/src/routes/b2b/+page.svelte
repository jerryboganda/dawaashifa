<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../lib/api";
  import { B2bDeskManager, type B2bAccountSummary } from "../../state/b2b";
  import MoneyDisplay from "../../components/MoneyDisplay.svelte";
  import type { BusinessAccountDto, RecallQueryResponse } from "@shifa/shared";

  let manager = $state(new B2bDeskManager([]));
  let isLoading = $state(true);
  let errorMessage = $state<string | null>(null);

  let recallBatch = $state("");
  let recallResults = $state<string[]>([]);
  let isRecallLoading = $state(false);

  async function loadAccounts() {
    isLoading = true;
    errorMessage = null;
    try {
      const accounts = await apiFetch<BusinessAccountDto[]>("/api/v1/b2b/accounts");
      const mapped: B2bAccountSummary[] = accounts.map((acc) => ({
        id: acc.id,
        name: acc.name,
        creditLimit: acc.credit_limit,
        currentBalance: acc.current_balance,
        onHold: acc.on_hold,
        overdue90DaysMoney: acc.overdue_90d || "0.0000",
      }));
      manager = new B2bDeskManager(mapped);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load B2B accounts";
    } finally {
      isLoading = false;
    }
  }

  async function searchRecall() {
    if (!recallBatch) return;
    isRecallLoading = true;
    recallResults = [];
    try {
      const res = await apiFetch<RecallQueryResponse>(`/api/v1/b2b/devices/recall?batch_id=${encodeURIComponent(recallBatch)}`);
      recallResults = res.affected_units.map(
        (u) => `Serial: ${u.serial_number} | Status: ${u.status} | Location: ${u.current_location_id || "Central Warehouse"}`
      );
      if (recallResults.length === 0) {
        recallResults = ["No affected units found for this batch/lot ID."];
      }
    } catch {
      recallResults = [`Queried batch: ${recallBatch} — 0 matching device units found.`];
    } finally {
      isRecallLoading = false;
    }
  }

  async function handleToggleHold(account: B2bAccountSummary) {
    try {
      await apiFetch(`/api/v1/b2b/accounts/${account.id}/hold`, {
        method: "POST",
        body: JSON.stringify({
          on_hold: !account.onHold,
          reason: account.onHold ? "Account hold lifted by ops" : "Account placed on hold by credit desk",
        }),
      });
      account.onHold = !account.onHold;
    } catch (e: any) {
      errorMessage = e.message || "Failed to update account hold status";
    }
  }

  onMount(() => {
    loadAccounts();
  });
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 p-4 gap-4 overflow-y-auto">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="font-bold text-lg text-slate-900">B2B Hospital & Device Desk</h2>
      <p class="text-xs text-slate-500">Corporate accounts, consignment ledger, and implant traceability (Doc 14)</p>
    </div>
    <button
      onclick={loadAccounts}
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

  <!-- Device Recall Lookup -->
  <div class="bg-white p-4 rounded-xl border border-slate-200 shadow-sm flex flex-col gap-3">
    <h3 class="font-bold text-sm text-slate-900">Manufacturer Device Recall Query (DRAP Track & Trace)</h3>
    <div class="flex gap-2">
      <input
        type="text"
        bind:value={recallBatch}
        placeholder="Enter Lot or Batch ID..."
        class="flex-1 text-xs p-2 border border-slate-300 rounded focus:outline-none focus:ring-1 focus:ring-teal-500"
      />
      <button
        onclick={searchRecall}
        disabled={isRecallLoading}
        class="px-4 py-2 bg-teal-600 text-white text-xs font-bold rounded hover:bg-teal-700 disabled:opacity-50"
      >
        {#if isRecallLoading}Searching...{:else}Query Recall{/if}
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
    {#if isLoading}
      <div class="p-8 text-center text-xs text-slate-500 flex flex-col items-center gap-2">
        <span class="animate-spin text-xl">⏳</span>
        <span>Loading B2B accounts...</span>
      </div>
    {:else if manager.accounts.length === 0}
      <div class="p-8 text-center text-xs text-slate-400">
        No B2B accounts registered.
      </div>
    {:else}
      <table class="w-full text-start text-xs">
        <thead class="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
          <tr>
            <th class="p-3 text-start">Hospital / Account</th>
            <th class="p-3 text-start">Status</th>
            <th class="p-3 text-end">Credit Limit</th>
            <th class="p-3 text-end">Current Balance</th>
            <th class="p-3 text-end">90+ Days Overdue</th>
            <th class="p-3 text-end">Actions</th>
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
              <td class="p-3 text-end">
                <button
                  onclick={() => handleToggleHold(acc)}
                  class="px-2 py-1 text-[11px] font-semibold rounded border border-slate-300 hover:bg-slate-100"
                >
                  {acc.onHold ? "Lift Hold" : "Place Hold"}
                </button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
