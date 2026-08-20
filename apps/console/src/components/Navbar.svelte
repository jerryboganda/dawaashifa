<script lang="ts">
  import { translations, type Locale } from "@shifa/shared";

  let { currentLocale = $bindable("en" as Locale), activeTab = $bindable("inbox") } = $props<{
    currentLocale?: Locale;
    activeTab?: string;
  }>();

  let t = $derived(translations[currentLocale]);
  let isRtl = $derived(translations[currentLocale].dir === "rtl");
</script>

<header class="bg-slate-900 text-white px-4 py-2 flex items-center justify-between border-b border-slate-800" dir={isRtl ? "rtl" : "ltr"}>
  <div class="flex items-center gap-6">
    <div class="flex items-center gap-2">
      <span class="w-8 h-8 rounded-md bg-teal-500 flex items-center justify-center font-bold text-slate-900 text-lg">
        ش
      </span>
      <span class="font-bold tracking-wide text-lg text-white">Shifa Ops</span>
    </div>

    <nav class="flex items-center gap-1 text-sm font-medium">
      <button
        onclick={() => (activeTab = "inbox")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'inbox' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
        {t.inbox.title}
      </button>
      <button
        onclick={() => (activeTab = "rx-review")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'rx-review' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
        {t.rxReview.title}
      </button>
      <button
        onclick={() => (activeTab = "payments")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'payments' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
        {t.payments.title}
      </button>
      <button
        onclick={() => (activeTab = "orders")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'orders' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
        {t.orders.title}
      </button>
      <button
        onclick={() => (activeTab = "inventory")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'inventory' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
        Inventory
      </button>
      <button
        onclick={() => (activeTab = "b2b")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'b2b' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
        {t.b2b.title}
      </button>
      <button
        onclick={() => (activeTab = "audit")}
        class="px-3 py-1.5 rounded-md transition-colors {activeTab === 'audit' ? 'bg-teal-600 text-white' : 'text-slate-300 hover:bg-slate-800'}"
      >
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
