<script lang="ts">
  import { onMount } from "svelte";
  import { apiFetch } from "../../lib/api";
  import type { AuditEventDto } from "@shifa/shared";

  let events = $state<AuditEventDto[]>([]);
  let isLoading = $state(true);
  let errorMessage = $state<string | null>(null);

  let selectedEntityType = $state("");
  let searchAction = $state("");

  async function loadEvents() {
    isLoading = true;
    errorMessage = null;
    try {
      let queryParams = new URLSearchParams();
      if (selectedEntityType) queryParams.set("entity_type", selectedEntityType);
      if (searchAction) queryParams.set("action", searchAction);

      const qs = queryParams.toString();
      const path = qs ? `/api/v1/admin/audit?${qs}` : "/api/v1/admin/audit";
      events = await apiFetch<AuditEventDto[]>(path);
    } catch (e: any) {
      errorMessage = e.message || "Failed to load regulatory audit logs";
    } finally {
      isLoading = false;
    }
  }

  async function handleExportCsv() {
    try {
      let queryParams = new URLSearchParams();
      if (selectedEntityType) queryParams.set("entity_type", selectedEntityType);
      if (searchAction) queryParams.set("action", searchAction);

      const qs = queryParams.toString();
      const path = qs ? `/api/v1/admin/audit/export?${qs}` : "/api/v1/admin/audit/export";
      const csv = await apiFetch<string>(path);

      const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.setAttribute("href", url);
      link.setAttribute("download", `drap_audit_export_${new Date().toISOString().slice(0, 10)}.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
    } catch (e: any) {
      errorMessage = e.message || "Failed to export audit CSV";
    }
  }

  onMount(() => {
    loadEvents();
  });
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 p-4 gap-4 overflow-y-auto">
  <div class="flex flex-wrap items-center justify-between gap-3">
    <div>
      <h2 class="font-bold text-lg text-slate-900">Regulatory Audit Explorer</h2>
      <p class="text-xs text-slate-500">Immutable ledger log for DRAP compliance and regulatory inspection (Invariant I-9)</p>
    </div>
    <div class="flex items-center gap-2">
      <select
        bind:value={selectedEntityType}
        onchange={loadEvents}
        class="bg-white border border-slate-300 text-xs rounded px-2.5 py-1.5 focus:outline-none focus:ring-1 focus:ring-slate-500"
      >
        <option value="">All Entities</option>
        <option value="PRESCRIPTION">Prescription</option>
        <option value="ORDER">Order</option>
        <option value="PAYMENT">Payment</option>
        <option value="FULFILMENT">Fulfilment</option>
        <option value="B2B_ACCOUNT">B2B Account</option>
        <option value="MESSAGE">Message / AI Override</option>
      </select>

      <button
        onclick={loadEvents}
        class="px-3 py-1.5 bg-white border border-slate-300 text-slate-700 text-xs font-semibold rounded hover:bg-slate-50"
      >
        🔄 Refresh
      </button>

      <button
        onclick={handleExportCsv}
        class="px-3 py-1.5 bg-slate-800 text-white text-xs font-semibold rounded hover:bg-slate-900"
      >
        📥 Export CSV
      </button>
    </div>
  </div>

  {#if errorMessage}
    <div class="bg-red-50 border border-red-200 text-red-700 text-xs p-3 rounded-lg flex items-center justify-between">
      <span>⚠️ {errorMessage}</span>
      <button onclick={() => (errorMessage = null)} class="font-bold hover:text-red-900">✕</button>
    </div>
  {/if}

  <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
    {#if isLoading}
      <div class="p-8 text-center text-xs text-slate-500 flex flex-col items-center gap-2">
        <span class="animate-spin text-xl">⏳</span>
        <span>Loading immutable audit records...</span>
      </div>
    {:else if events.length === 0}
      <div class="p-8 text-center text-xs text-slate-400">
        No audit log events found matching the filter criteria.
      </div>
    {:else}
      <table class="w-full text-start text-xs">
        <thead class="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
          <tr>
            <th class="p-3 text-start">Timestamp (UTC)</th>
            <th class="p-3 text-start">Actor</th>
            <th class="p-3 text-start">Entity</th>
            <th class="p-3 text-start">Action</th>
            <th class="p-3 text-start">State Change Diff / Details</th>
          </tr>
        </thead>
        <tbody class="divide-y divide-slate-100">
          {#each events as ev}
            <tr class="hover:bg-slate-50">
              <td class="p-3 font-mono text-slate-500 whitespace-nowrap">{ev.occurred_at}</td>
              <td class="p-3 font-semibold text-slate-900">
                <span class="text-[10px] bg-slate-100 text-slate-600 px-1 py-0.5 rounded font-mono mr-1">{ev.actor_type}</span>
                {ev.actor_id || "SYSTEM"}
              </td>
              <td class="p-3 font-mono text-slate-700">{ev.entity_type} ({ev.entity_id})</td>
              <td class="p-3 font-bold text-teal-700">{ev.action}</td>
              <td class="p-3 font-mono text-[11px]">
                {#if ev.before}
                  <div class="text-red-700 bg-red-50 p-1 rounded mb-1">- {JSON.stringify(ev.before)}</div>
                {/if}
                {#if ev.after}
                  <div class="text-emerald-700 bg-emerald-50 p-1 rounded">+ {JSON.stringify(ev.after)}</div>
                {/if}
                {#if ev.reason}
                  <div class="text-slate-500 italic mt-0.5">Reason: {ev.reason}</div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
