<script lang="ts">
  interface AuditEvent {
    id: string;
    actor: string;
    entityType: string;
    entityId: string;
    action: string;
    timestamp: string;
    beforeState: string;
    afterState: string;
  }

  const mockAudit: AuditEvent[] = [
    {
      id: "aud-1",
      actor: "pharmacist_dr_tariq",
      entityType: "PRESCRIPTION",
      entityId: "RX-2026-901",
      action: "APPROVE",
      timestamp: "2026-08-20 10:15:30 UTC",
      beforeState: '{"status": "RX_UNDER_REVIEW"}',
      afterState: '{"status": "CONFIRMED", "approval_user_id": "usr-101"}',
    },
    {
      id: "aud-2",
      actor: "cfo_admin",
      entityType: "B2B_ACCOUNT",
      entityId: "ACC-SERVICES-HOSP",
      action: "CREDIT_OVERRIDE",
      timestamp: "2026-08-20 11:20:10 UTC",
      beforeState: '{"credit_limit": 5000000}',
      afterState: '{"credit_override": 6000000, "reason": "VIP cardiac surgery"}',
    },
  ];

  let events = $state(mockAudit);
</script>

<div class="flex flex-col h-[calc(100vh-50px)] bg-slate-100 p-4 gap-4 overflow-y-auto">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="font-bold text-lg text-slate-900">Regulatory Audit Explorer</h2>
      <p class="text-xs text-slate-500">Immutable ledger log for DRAP compliance and regulatory inspection</p>
    </div>
    <button class="px-3 py-1.5 bg-slate-800 text-white text-xs font-semibold rounded hover:bg-slate-900">
      📥 Export CSV
    </button>
  </div>

  <div class="bg-white rounded-xl border border-slate-200 overflow-hidden shadow-sm">
    <table class="w-full text-start text-xs">
      <thead class="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
        <tr>
          <th class="p-3 text-start">Timestamp (UTC)</th>
          <th class="p-3 text-start">Actor</th>
          <th class="p-3 text-start">Entity</th>
          <th class="p-3 text-start">Action</th>
          <th class="p-3 text-start">State Change Diff</th>
        </tr>
      </thead>
      <tbody class="divide-y divide-slate-100">
        {#each events as ev}
          <tr class="hover:bg-slate-50">
            <td class="p-3 font-mono text-slate-500">{ev.timestamp}</td>
            <td class="p-3 font-semibold text-slate-900">{ev.actor}</td>
            <td class="p-3 font-mono">{ev.entityType} ({ev.entityId})</td>
            <td class="p-3 font-bold text-teal-700">{ev.action}</td>
            <td class="p-3 font-mono text-[11px]">
              <div class="text-red-700 bg-red-50 p-1 rounded mb-1">- {ev.beforeState}</div>
              <div class="text-emerald-700 bg-emerald-50 p-1 rounded">+ {ev.afterState}</div>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</div>
