<script lang="ts">
  import { translations, type Locale } from "@shifa/shared";

  let { currentLocale = $bindable("en" as Locale), activeTab = $bindable("inbox") } = $props<{
    currentLocale?: Locale;
    activeTab?: string;
  }>();

  let t = $derived(translations[currentLocale]);
  let isRtl = $derived(translations[currentLocale].dir === "rtl");

  function getNavBtnClass(tab: string) {
    if (activeTab === tab) {
      return "px-3 py-1.5 rounded-md transition-colors bg-teal-600 text-white font-semibold shadow-xs";
    }
    return "px-3 py-1.5 rounded-md transition-colors text-slate-300 hover:text-white hover:bg-slate-800";
  }
</script>

<header class="bg-slate-900 text-white px-4 py-2 flex items-center justify-between border-b border-slate-800" dir={isRtl ? "rtl" : "ltr"}>
  <div class="flex items-center gap-6">
    <div class="flex items-center gap-2">
      <span class="w-8 h-8 rounded-md bg-teal-500 flex items-center justify-center font-black text-white text-lg shadow-xs">
        ش
      </span>
      <span class="font-bold tracking-wide text-lg text-white">Shifa Ops</span>
    </div>

    <nav class="flex items-center gap-1 text-sm font-medium">
      <button onclick={() => (activeTab = "inbox")} class={getNavBtnClass("inbox")}>
        {t.inbox.title}
      </button>
      <button onclick={() => (activeTab = "rx-review")} class={getNavBtnClass("rx-review")}>
        {t.rxReview.title}
      </button>
      <button onclick={() => (activeTab = "payments")} class={getNavBtnClass("payments")}>
        {t.payments.title}
      </button>
      <button onclick={() => (activeTab = "orders")} class={getNavBtnClass("orders")}>
        {t.orders.title}
      </button>
      <button onclick={() => (activeTab = "inventory")} class={getNavBtnClass("inventory")}>
        Inventory
      </button>
      <button onclick={() => (activeTab = "b2b")} class={getNavBtnClass("b2b")}>
        {t.b2b.title}
      </button>
      <button onclick={() => (activeTab = "audit")} class={getNavBtnClass("audit")}>
        Audit Explorer
      </button>
    </nav>
  </div>

  <div class="flex items-center gap-3">
    <!-- Language Switcher -->
    <select
      bind:value={currentLocale}
      class="bg-slate-800 text-xs text-white border border-slate-700 rounded px-2 py-1 focus:outline-none focus:ring-1 focus:ring-teal-500"
    >
      <option value="en">English (EN)</option>
      <option value="ur">اردو (UR)</option>
      <option value="ur-Latn">Roman Urdu</option>
    </select>
  </div>
</header>
