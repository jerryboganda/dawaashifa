<script lang="ts">
  import { formatPkr } from "@shifa/shared";

  let { amount = "0.0000", customClass = "" } = $props<{
    amount?: string | null;
    customClass?: string;
  }>();

  let formatted = $derived(formatPkr(amount));
</script>

<span class="font-mono font-medium tracking-tight {customClass}">
  {formatted}
</span>
