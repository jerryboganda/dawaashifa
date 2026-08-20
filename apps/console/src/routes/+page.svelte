<script lang="ts">
  import Navbar from "../components/Navbar.svelte";
  import InboxPage from "./inbox/+page.svelte";
  import RxReviewPage from "./rx-review/+page.svelte";
  import PaymentReviewPage from "./payments/review/+page.svelte";
  import OrderBoardPage from "./orders/+page.svelte";
  import InventoryPage from "./inventory/+page.svelte";
  import B2bPage from "./b2b/+page.svelte";
  import AuditPage from "./audit/+page.svelte";
  import { type Locale } from "@shifa/shared";

  let currentLocale = $state<Locale>("en");
  let activeTab = $state("inbox");
</script>

<div class="min-h-screen flex flex-col font-sans">
  <Navbar bind:currentLocale bind:activeTab />

  <div class="flex-1">
    {#if activeTab === "inbox"}
      <InboxPage {currentLocale} />
    {:else if activeTab === "rx-review"}
      <RxReviewPage {currentLocale} />
    {:else if activeTab === "payments"}
      <PaymentReviewPage {currentLocale} />
    {:else if activeTab === "orders"}
      <OrderBoardPage {currentLocale} />
    {:else if activeTab === "inventory"}
      <InventoryPage />
    {:else if activeTab === "b2b"}
      <B2bPage />
    {:else if activeTab === "audit"}
      <AuditPage />
    {/if}
  </div>
</div>
