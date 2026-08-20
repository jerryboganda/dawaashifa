<script lang="ts">
  import { colors } from "@shifa/shared";

  let { status = "pending", label = "" } = $props<{
    status?: keyof typeof colors.status | string;
    label?: string;
  }>();

  const statusMap: Record<string, { bg: string; text: string }> = {
    pending: { bg: "bg-amber-100", text: "text-amber-800" },
    review: { bg: "bg-blue-100", text: "text-blue-800" },
    approved: { bg: "bg-emerald-100", text: "text-emerald-800" },
    rejected: { bg: "bg-red-100", text: "text-red-800" },
    dispatched: { bg: "bg-purple-100", text: "text-purple-800" },
    delivered: { bg: "bg-emerald-200", text: "text-emerald-900" },
    failed: { bg: "bg-red-200", text: "text-red-900" },
  };

  let style = $derived(statusMap[status.toLowerCase()] || { bg: "bg-slate-100", text: "text-slate-800" });
</script>

<span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold uppercase tracking-wider {style.bg} {style.text}">
  {label || status}
</span>
