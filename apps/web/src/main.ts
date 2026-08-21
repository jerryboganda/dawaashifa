import { formatPkr } from "@shifa/shared";

// ------------------------------------------------------------------------------------------------
// Sample Authenticated Drug Catalog (DRAP Compliant with MRP)
// ------------------------------------------------------------------------------------------------
export interface CatalogItem {
  id: string;
  name: string;
  generic: string;
  category: "CHRONIC" | "OTC" | "ANTIBIOTIC" | "DEVICE";
  mrp: string;
  isRx: boolean;
  packSize: string;
  manufacturer: string;
}

export const PRODUCTS: CatalogItem[] = [
  {
    id: "prod-1",
    name: "Panadol Extra 500mg/65mg",
    generic: "Paracetamol + Caffeine",
    category: "OTC",
    mrp: "450.00",
    isRx: false,
    packSize: "Box of 100 Tablets",
    manufacturer: "GSK Pakistan",
  },
  {
    id: "prod-2",
    name: "Glucophage 500mg",
    generic: "Metformin Hydrochloride",
    category: "CHRONIC",
    mrp: "385.50",
    isRx: true,
    packSize: "Box of 50 Tablets",
    manufacturer: "Martin Dow",
  },
  {
    id: "prod-3",
    name: "Augmentin 625mg",
    generic: "Amoxicillin + Clavulanic Acid",
    category: "ANTIBIOTIC",
    mrp: "745.00",
    isRx: true,
    packSize: "Box of 14 Tablets",
    manufacturer: "GSK Pakistan",
  },
  {
    id: "prod-4",
    name: "Accu-Chek Instant Blood Glucose Meter",
    generic: "Glucometer with 10 Strips",
    category: "DEVICE",
    mrp: "3200.00",
    isRx: false,
    packSize: "1 Device Kit",
    manufacturer: "Roche Diagnostics",
  },
  {
    id: "prod-5",
    name: "Concor 5mg",
    generic: "Bisoprolol Fumarate",
    category: "CHRONIC",
    mrp: "580.00",
    isRx: true,
    packSize: "Box of 30 Tablets",
    manufacturer: "Merck Serono",
  },
  {
    id: "prod-6",
    name: "Disprin Direct 300mg",
    generic: "Aspirin Dispersible",
    category: "OTC",
    mrp: "120.00",
    isRx: false,
    packSize: "Strip of 10 Tablets",
    manufacturer: "Reckitt Benckiser",
  },
  {
    id: "prod-7",
    name: "Lipitor 20mg",
    generic: "Atorvastatin Calcium",
    category: "CHRONIC",
    mrp: "1150.00",
    isRx: true,
    packSize: "Box of 30 Tablets",
    manufacturer: "Pfizer Pakistan",
  },
  {
    id: "prod-8",
    name: "Omeprazole 20mg (Risek)",
    generic: "Omeprazole Pellets",
    category: "OTC",
    mrp: "620.00",
    isRx: false,
    packSize: "Box of 14 Capsules",
    manufacturer: "Getz Pharma",
  },
];

// ------------------------------------------------------------------------------------------------
// DOM Rendering & Event Listeners
// ------------------------------------------------------------------------------------------------
if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", () => {
    renderProducts(PRODUCTS);
    setupEventListeners();
  });
}

export function setupEventListeners() {
  if (typeof document === "undefined") return;

  // Category filter
  const catFilter = document.getElementById("cat-filter") as HTMLSelectElement | null;
  if (catFilter) {
    catFilter.addEventListener("change", (e) => {
      const selected = (e.target as HTMLSelectElement).value;
      if (selected === "ALL") {
        renderProducts(PRODUCTS);
      } else {
        renderProducts(PRODUCTS.filter((p) => p.category === selected));
      }
    });
  }

  // Quick tracking lookup
  const trackBtn = document.getElementById("btn-quick-track");
  const trackInput = document.getElementById("input-tracking") as HTMLInputElement | null;
  const trackResult = document.getElementById("track-result-box");

  if (trackBtn && trackInput && trackResult) {
    trackBtn.addEventListener("click", () => {
      const val = trackInput.value.trim();
      if (!val) {
        trackResult.classList.remove("hidden");
        trackResult.innerHTML = `<span class="text-amber-400 font-bold">Please enter tracking number or phone number</span>`;
        return;
      }
      trackResult.classList.remove("hidden");
      trackResult.innerHTML = `
        <div class="space-y-1">
          <div class="flex items-center justify-between font-bold">
            <span>Order #${val}</span>
            <span class="text-emerald-400">OUT FOR DELIVERY</span>
          </div>
          <p class="text-slate-300 text-[11px]">Rider Tariq Mahmood is on the way (Cold-Chain Box #12). Estimated: 18 mins.</p>
        </div>
      `;
    });
  }

  // Modal interactions
  const rxDialog = document.getElementById("rx-dialog") as HTMLDialogElement | null;
  const openModalBtn = document.getElementById("btn-upload-rx-modal");
  const closeModalBtn = document.getElementById("btn-close-modal");
  const submitRxBtn = document.getElementById("btn-submit-rx");

  if (openModalBtn && rxDialog) {
    openModalBtn.addEventListener("click", () => rxDialog.showModal());
  }
  if (closeModalBtn && rxDialog) {
    closeModalBtn.addEventListener("click", () => rxDialog.close());
  }
  if (submitRxBtn && rxDialog) {
    submitRxBtn.addEventListener("click", () => {
      alert("Prescription submitted! Our pharmacist will message you on WhatsApp within 3 minutes.");
      rxDialog.close();
    });
  }

  // Language switcher
  setupLanguageSwitcher();
}

function renderProducts(items: CatalogItem[]) {
  const container = document.getElementById("product-grid");
  if (!container) return;

  container.innerHTML = items
    .map(
      (p) => `
    <div class="bg-white rounded-2xl p-5 border border-slate-200 shadow-xs hover:shadow-md transition-shadow flex flex-col justify-between group">
      <div>
        <div class="flex items-center justify-between mb-3">
          <span class="px-2 py-0.5 rounded-full text-[10px] font-extrabold ${
            p.isRx
              ? "bg-amber-100 text-amber-800 border border-amber-300"
              : "bg-emerald-100 text-emerald-800 border border-emerald-300"
          }">
            ${p.isRx ? "PRESCRIPTION REQUIRED" : "OTC AVAILABLE"}
          </span>
          <span class="text-[11px] font-semibold text-slate-400">${p.packSize}</span>
        </div>

        <h4 class="font-bold text-slate-900 text-base group-hover:text-teal-700 transition-colors">${p.name}</h4>
        <p class="text-xs text-slate-500 italic mt-0.5">${p.generic}</p>
        <p class="text-[11px] text-slate-400 mt-1">Mfg: ${p.manufacturer}</p>
      </div>

      <div class="mt-6 pt-4 border-t border-slate-100 flex items-center justify-between">
        <div>
          <span class="block text-[10px] uppercase font-bold text-slate-400">MRP (Govt Fixed)</span>
          <span class="text-base font-extrabold text-slate-900">${formatPkr(p.mrp)}</span>
        </div>

        <a href="https://wa.me/923000000000?text=I%20want%20to%20order%20${encodeURIComponent(p.name)}" target="_blank" rel="noopener noreferrer" class="px-3.5 py-2 rounded-xl bg-teal-50 hover:bg-teal-600 text-teal-700 hover:text-white font-bold text-xs transition-colors flex items-center gap-1.5">
          <span>Order</span>
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/></svg>
        </a>
      </div>
    </div>
  `,
    )
    .join("");
}

function setupLanguageSwitcher() {
  const enBtn = document.getElementById("lang-en");
  const urBtn = document.getElementById("lang-ur");
  const latnBtn = document.getElementById("lang-ur-latn");

  const setActive = (activeBtn: HTMLElement | null) => {
    [enBtn, urBtn, latnBtn].forEach((btn) => {
      if (btn) {
        btn.className =
          btn === activeBtn
            ? "px-2 py-0.5 rounded bg-teal-700 text-white shadow-sm font-bold"
            : "px-2 py-0.5 rounded hover:bg-teal-700/50 text-teal-200 font-medium";
      }
    });
  };

  if (enBtn) {
    enBtn.addEventListener("click", () => {
      document.documentElement.setAttribute("dir", "ltr");
      document.documentElement.setAttribute("lang", "en");
      setActive(enBtn);
    });
  }

  if (urBtn) {
    urBtn.addEventListener("click", () => {
      document.documentElement.setAttribute("dir", "rtl");
      document.documentElement.setAttribute("lang", "ur");
      setActive(urBtn);
    });
  }

  if (latnBtn) {
    latnBtn.addEventListener("click", () => {
      document.documentElement.setAttribute("dir", "ltr");
      document.documentElement.setAttribute("lang", "ur-Latn");
      setActive(latnBtn);
    });
  }
}
