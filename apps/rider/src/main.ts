import "./app.css";
import { formatPkr } from "@shifa/shared";
import { translations, isRTL, type Language } from "./lib/i18n";
import { offlineQueue } from "./lib/idb";

// ------------------------------------------------------------------------------------------------
// Rider Dashboard Interactive PWA
// ------------------------------------------------------------------------------------------------
let currentLang: Language = "en";
let onShift = true;
let activeTab: "deliveries" | "cash" = "deliveries";

interface MockDelivery {
  id: string;
  orderRef: string;
  customerName: string;
  phone: string;
  address: string;
  status: "ALLOCATED" | "PICKED_UP" | "IN_TRANSIT" | "DELIVERED" | "FAILED";
  codAmount: string;
  isControlled: boolean;
  items: string[];
}

let deliveries: MockDelivery[] = [
  {
    id: "del-101",
    orderRef: "ORD-9821",
    customerName: "Mohammad Usman",
    phone: "+923001234567",
    address: "House 42-A, Sector F-8/2, Islamabad",
    status: "ALLOCATED",
    codAmount: "1450.0000",
    isControlled: false,
    items: ["Panadol Extra x 2", "Disprin 300mg x 1"],
  },
  {
    id: "del-102",
    orderRef: "ORD-9825",
    customerName: "Dr. Fatima Hashmi",
    phone: "+923219876543",
    address: "Apartment 5B, Silver Oaks, F-10, Islamabad",
    status: "PICKED_UP",
    codAmount: "3200.0000",
    isControlled: true,
    items: ["Rivotril 2mg x 1 (Rx)", "Augmentin 625mg x 2"],
  },
];

let selectedDeliveryForPod: MockDelivery | null = null;
let capturedPhotoData: string | null = null;
let capturedGps: { lat: number; lng: number } | null = null;
let useGpsDenied = false;

export function initRiderApp() {
  const root = document.getElementById("app");
  if (!root) return;

  renderApp(root);
}

function renderApp(container: HTMLElement) {
  const t = translations[currentLang];
  const rtl = isRTL(currentLang);
  document.documentElement.dir = rtl ? "rtl" : "ltr";
  document.documentElement.lang = currentLang;

  const pendingCount = offlineQueue.getPendingCount();
  const isOnline = typeof navigator !== "undefined" ? navigator.onLine : true;

  container.innerHTML = `
    <div class="min-h-screen bg-slate-900 text-slate-100 flex flex-col max-w-lg mx-auto border-x border-slate-800 shadow-2xl">
      
      <!-- Top Status Header -->
      <header class="p-4 bg-slate-950 border-b border-slate-800 flex items-center justify-between sticky top-0 z-30">
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-xl bg-emerald-600 flex items-center justify-center font-black text-white text-base shadow-md shadow-emerald-950">
            🚴
          </div>
          <div>
            <h1 class="font-extrabold text-base tracking-tight text-white">${t.appTitle}</h1>
            <div class="flex items-center gap-2 text-xs">
              <span class="inline-flex items-center gap-1 ${onShift ? 'text-emerald-400 font-bold' : 'text-slate-400'}">
                <span class="w-2 h-2 rounded-full ${onShift ? 'bg-emerald-400 animate-pulse' : 'bg-slate-500'}"></span>
                ${onShift ? t.onShift : t.offShift}
              </span>
              <span class="text-slate-600">•</span>
              <span class="${isOnline ? 'text-teal-400' : 'text-amber-400'} font-medium">
                ${isOnline ? t.online : t.offline}
              </span>
            </div>
          </div>
        </div>

        <!-- Header Actions: Language & Shift -->
        <div class="flex items-center gap-2">
          <select id="rider-lang" class="bg-slate-800 text-xs text-white border border-slate-700 rounded-lg px-2 py-1.5 focus:outline-none focus:ring-1 focus:ring-emerald-500">
            <option value="en" ${currentLang === 'en' ? 'selected' : ''}>EN</option>
            <option value="ur" ${currentLang === 'ur' ? 'selected' : ''}>اردو</option>
            <option value="roman_ur" ${currentLang === 'roman_ur' ? 'selected' : ''}>Roman</option>
          </select>
          <button id="toggle-shift" class="px-3 py-1.5 rounded-lg text-xs font-bold transition-colors ${onShift ? 'bg-rose-900/80 hover:bg-rose-800 text-rose-200 border border-rose-700/50' : 'bg-emerald-600 hover:bg-emerald-500 text-white'}">
            ${onShift ? t.endShift : t.startShift}
          </button>
        </div>
      </header>

      <!-- Offline Sync Banner if queued -->
      ${pendingCount > 0 ? `
        <div class="bg-amber-500/20 border-b border-amber-500/30 px-4 py-2 text-amber-300 text-xs font-semibold flex items-center justify-between">
          <span>⚠️ ${pendingCount} ${t.syncPending}</span>
          <button id="btn-sync-now" class="px-2 py-0.5 rounded bg-amber-600 text-white font-bold text-[11px]">Sync</button>
        </div>
      ` : ''}

      <!-- Navigation Tabs -->
      <nav class="flex border-b border-slate-800 bg-slate-950/60 text-xs font-bold">
        <button id="tab-del" class="flex-1 py-3 text-center border-b-2 transition-colors ${activeTab === 'deliveries' ? 'border-emerald-500 text-emerald-400 bg-emerald-500/10' : 'border-transparent text-slate-400 hover:text-slate-200'}">
          ${t.deliveriesTab} (${deliveries.filter(d => d.status !== 'DELIVERED').length})
        </button>
        <button id="tab-cash" class="flex-1 py-3 text-center border-b-2 transition-colors ${activeTab === 'cash' ? 'border-emerald-500 text-emerald-400 bg-emerald-500/10' : 'border-transparent text-slate-400 hover:text-slate-200'}">
          ${t.cashTab}
        </button>
      </nav>

      <!-- Main Content -->
      <main class="flex-1 p-4 overflow-y-auto space-y-4">
        ${activeTab === 'deliveries' ? renderDeliveriesTab(t) : renderCashTab(t)}
      </main>

      <!-- POD Modal -->
      <div id="pod-modal-container"></div>
    </div>
  `;

  attachEventHandlers(container);
}

function renderDeliveriesTab(t: typeof translations["en"]) {
  if (deliveries.length === 0) {
    return `<div class="text-center py-12 text-slate-500 text-sm">No deliveries assigned right now.</div>`;
  }

  return deliveries
    .map(
      (del) => `
    <div class="bg-slate-950 border border-slate-800 rounded-2xl p-4 space-y-3 shadow-lg">
      <div class="flex items-center justify-between border-b border-slate-800/80 pb-2.5">
        <div>
          <span class="text-xs font-mono text-slate-400 font-bold">${del.orderRef}</span>
          <h2 class="text-sm font-extrabold text-white mt-0.5">${del.customerName}</h2>
        </div>
        <span class="px-2.5 py-1 rounded-full text-[10px] font-extrabold ${getStatusBadgeClass(del.status)}">
          ${del.status}
        </span>
      </div>

      <div class="text-xs space-y-1.5 text-slate-300">
        <p class="flex items-start gap-1.5">
          <span class="text-slate-500">📍</span>
          <span>${del.address}</span>
        </p>
        <div class="flex items-center justify-between pt-1">
          <span class="text-slate-400 font-medium">${t.codAmount}:</span>
          <span class="text-sm font-black text-emerald-400">${formatPkr(del.codAmount)}</span>
        </div>
        ${del.isControlled ? `
          <div class="p-2 rounded-xl bg-amber-500/10 border border-amber-500/30 text-amber-300 text-[11px] font-bold">
            ⚠️ ${t.controlledSubstanceWarning}
          </div>
        ` : ''}
      </div>

      <!-- Quick Action Buttons -->
      <div class="flex flex-wrap items-center gap-2 pt-2 border-t border-slate-800/80">
        <a href="tel:${del.phone}" class="flex-1 min-h-[44px] flex items-center justify-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-bold rounded-xl transition-colors">
          📞 ${t.callCustomer}
        </a>
        <a href="https://wa.me/${del.phone.replace(/[^0-9]/g, '')}" target="_blank" class="flex-1 min-h-[44px] flex items-center justify-center gap-1.5 px-3 py-2 bg-emerald-700/60 hover:bg-emerald-600 text-emerald-100 text-xs font-bold rounded-xl transition-colors">
          💬 ${t.whatsappCustomer}
        </a>
      </div>

      <!-- Workflow Progression Buttons -->
      <div class="pt-1">
        ${renderDeliveryActionButton(del, t)}
      </div>
    </div>
  `,
    )
    .join("");
}

function renderDeliveryActionButton(del: MockDelivery, t: typeof translations["en"]) {
  if (del.status === "ALLOCATED") {
    return `
      <button data-action="pickup" data-id="${del.id}" class="w-full min-h-[48px] bg-teal-600 hover:bg-teal-500 text-white font-extrabold text-xs rounded-xl shadow-md transition-colors">
        ${t.pickedUp}
      </button>
    `;
  }
  if (del.status === "PICKED_UP") {
    return `
      <button data-action="start-transit" data-id="${del.id}" class="w-full min-h-[48px] bg-blue-600 hover:bg-blue-500 text-white font-extrabold text-xs rounded-xl shadow-md transition-colors">
        ${t.startDelivery}
      </button>
    `;
  }
  if (del.status === "IN_TRANSIT") {
    return `
      <button data-action="open-pod" data-id="${del.id}" class="w-full min-h-[48px] bg-emerald-600 hover:bg-emerald-500 text-white font-extrabold text-xs rounded-xl shadow-md transition-colors">
        ${t.deliverPod}
      </button>
    `;
  }
  if (del.status === "DELIVERED") {
    return `<div class="text-center py-2 text-xs font-bold text-emerald-400">✓ Completed & Logged</div>`;
  }
  return "";
}

function renderCashTab(t: typeof translations["en"]) {
  const totalCod = deliveries
    .filter(d => d.status === "DELIVERED")
    .reduce((acc, d) => acc + parseFloat(d.codAmount || "0"), 0);

  return `
    <div class="bg-slate-950 border border-slate-800 rounded-2xl p-5 space-y-4 shadow-xl">
      <h2 class="font-extrabold text-sm text-white">${t.cashReconTitle}</h2>

      <div class="p-4 rounded-xl bg-slate-900 border border-slate-800 space-y-2">
        <span class="text-xs text-slate-400 block">${t.cashExpectedToday}</span>
        <span class="text-2xl font-black text-emerald-400">${formatPkr(totalCod.toFixed(4))}</span>
      </div>

      <div class="space-y-3">
        <label for="input-declare-cash" class="block text-xs font-bold text-slate-300">${t.declareCollectedAmount}</label>
        <input type="number" id="input-declare-cash" value="${totalCod > 0 ? totalCod : ''}" placeholder="e.g. 4650" class="w-full min-h-[44px] bg-slate-900 border border-slate-700 rounded-xl px-4 py-2.5 text-sm text-white focus:outline-none focus:border-emerald-500" />
        
        <button id="btn-submit-cash" class="w-full min-h-[48px] bg-emerald-600 hover:bg-emerald-500 text-white font-extrabold text-sm rounded-xl shadow-md transition-colors">
          ${t.submitDeclaration}
        </button>
      </div>

      <div id="cash-status-box" class="hidden p-3 rounded-xl bg-emerald-950/60 border border-emerald-500/30 text-emerald-300 text-xs font-semibold">
        ✓ Shift cash declaration submitted to Branch Cashier. Status: PENDING RECONCILIATION.
      </div>
    </div>
  `;
}

function getStatusBadgeClass(status: MockDelivery["status"]) {
  switch (status) {
    case "ALLOCATED":
      return "bg-amber-500/20 text-amber-300 border border-amber-500/30";
    case "PICKED_UP":
      return "bg-blue-500/20 text-blue-300 border border-blue-500/30";
    case "IN_TRANSIT":
      return "bg-purple-500/20 text-purple-300 border border-purple-500/30";
    case "DELIVERED":
      return "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30";
    case "FAILED":
      return "bg-rose-500/20 text-rose-300 border border-rose-500/30";
  }
}

function attachEventHandlers(container: HTMLElement) {
  // Lang switcher
  const langSel = container.querySelector("#rider-lang") as HTMLSelectElement | null;
  if (langSel) {
    langSel.addEventListener("change", (e) => {
      currentLang = (e.target as HTMLSelectElement).value as Language;
      renderApp(container);
    });
  }

  // Toggle shift
  const shiftBtn = container.querySelector("#toggle-shift");
  if (shiftBtn) {
    shiftBtn.addEventListener("click", () => {
      onShift = !onShift;
      renderApp(container);
    });
  }

  // Tabs
  const tabDel = container.querySelector("#tab-del");
  const tabCash = container.querySelector("#tab-cash");
  if (tabDel) {
    tabDel.addEventListener("click", () => {
      activeTab = "deliveries";
      renderApp(container);
    });
  }
  if (tabCash) {
    tabCash.addEventListener("click", () => {
      activeTab = "cash";
      renderApp(container);
    });
  }

  // Action buttons
  container.querySelectorAll("button[data-action]").forEach((btn) => {
    btn.addEventListener("click", (e) => {
      const target = e.currentTarget as HTMLElement;
      const action = target.getAttribute("data-action");
      const id = target.getAttribute("data-id");
      const d = deliveries.find((x) => x.id === id);
      if (!d) return;

      if (action === "pickup") {
        d.status = "PICKED_UP";
        renderApp(container);
      } else if (action === "start-transit") {
        d.status = "IN_TRANSIT";
        renderApp(container);
      } else if (action === "open-pod") {
        selectedDeliveryForPod = d;
        openPodModal(container, d);
      }
    });
  });

  // Cash declaration
  const submitCashBtn = container.querySelector("#btn-submit-cash");
  const cashStatus = container.querySelector("#cash-status-box");
  if (submitCashBtn && cashStatus) {
    submitCashBtn.addEventListener("click", () => {
      cashStatus.classList.remove("hidden");
    });
  }
}

function openPodModal(container: HTMLElement, del: MockDelivery) {
  const modalContainer = container.querySelector("#pod-modal-container");
  if (!modalContainer) return;

  const t = translations[currentLang];

  modalContainer.innerHTML = `
    <div class="fixed inset-0 z-50 bg-slate-950/80 backdrop-blur-sm flex items-end sm:items-center justify-center p-4">
      <div class="bg-slate-900 border border-slate-800 rounded-3xl p-6 w-full max-w-md space-y-4 shadow-2xl">
        <div class="flex items-center justify-between border-b border-slate-800 pb-3">
          <h3 class="font-extrabold text-base text-white">${t.podTitle} (${del.orderRef})</h3>
          <button id="close-pod-modal" class="text-slate-400 hover:text-white font-bold text-lg p-1">✕</button>
        </div>

        <div class="space-y-3 text-xs">
          <!-- Mandatory Photo Dropzone / Capture -->
          <div>
            <label class="block font-bold text-slate-300 mb-1">${t.takePhoto}</label>
            <div id="pod-photo-box" class="border-2 border-dashed ${capturedPhotoData ? 'border-emerald-500 bg-emerald-500/10' : 'border-slate-700 bg-slate-950'} rounded-2xl p-4 text-center cursor-pointer min-h-[48px] flex items-center justify-center">
              <span class="${capturedPhotoData ? 'text-emerald-400 font-bold' : 'text-slate-400'}">
                ${capturedPhotoData ? '📸 ' + t.photoCaptured : '📷 ' + t.takePhoto}
              </span>
            </div>
          </div>

          <!-- Recipient Name -->
          <div>
            <label for="pod-name" class="block font-bold text-slate-300 mb-1">${t.recipientName}</label>
            <input type="text" id="pod-name" value="${del.customerName}" class="w-full min-h-[44px] bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-xs text-white focus:outline-none focus:border-emerald-500" />
          </div>

          <!-- Controlled Substance Invariants (Doc 12 §4) -->
          ${del.isControlled ? `
            <div class="p-3 bg-amber-500/10 border border-amber-500/30 rounded-xl space-y-2">
              <label class="flex items-start gap-2 cursor-pointer">
                <input type="checkbox" id="pod-rx-check" class="mt-0.5 w-4 h-4 rounded text-emerald-600 focus:ring-emerald-500" />
                <span class="text-amber-200 font-bold text-[11px]">${t.originalRxCollected}</span>
              </label>
              <div>
                <label for="pod-cnic" class="block font-bold text-slate-300 text-[11px] mb-1">${t.recipientCnicLast4}</label>
                <input type="text" id="pod-cnic" maxlength="4" placeholder="e.g. 4821" class="w-full min-h-[44px] bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-xs text-white font-mono focus:outline-none focus:border-emerald-500" />
              </div>
            </div>
          ` : ''}

          <!-- GPS Section -->
          <div class="p-3 bg-slate-950 rounded-xl border border-slate-800 space-y-1.5">
            <div class="flex items-center justify-between">
              <span class="font-semibold text-slate-300">GPS Location:</span>
              <span class="${capturedGps || useGpsDenied ? 'text-emerald-400 font-bold' : 'text-slate-500'}">
                ${capturedGps ? '✓ ' + t.gpsCaptured : (useGpsDenied ? '⚠️ GPS Denied Override' : 'Pending')}
              </span>
            </div>
            <div class="flex gap-2">
              <button id="btn-capture-gps" class="flex-1 min-h-[44px] py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 font-bold text-[11px] rounded-lg">
                ${t.captureGps}
              </button>
              <button id="btn-override-gps" class="flex-1 min-h-[44px] py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-400 font-medium text-[11px] rounded-lg">
                ${t.useGpsDeniedOverride}
              </button>
            </div>
          </div>

          <div id="pod-error-msg" class="hidden p-2 rounded-xl bg-rose-500/20 border border-rose-500/30 text-rose-300 font-bold text-[11px]"></div>

          <button id="btn-submit-pod" class="w-full min-h-[48px] bg-emerald-600 hover:bg-emerald-500 text-white font-extrabold text-sm rounded-xl shadow-md transition-colors">
            ${t.submitPod}
          </button>
        </div>
      </div>
    </div>
  `;

  // Attach modal listeners
  const closeBtn = modalContainer.querySelector("#close-pod-modal");
  if (closeBtn) {
    closeBtn.addEventListener("click", () => {
      modalContainer.innerHTML = "";
    });
  }

  const photoBox = modalContainer.querySelector("#pod-photo-box");
  if (photoBox) {
    photoBox.addEventListener("click", () => {
      capturedPhotoData = "data:image/jpeg;base64,mock_pod_parcel_image_sample";
      openPodModal(container, del);
    });
  }

  const gpsBtn = modalContainer.querySelector("#btn-capture-gps");
  if (gpsBtn) {
    gpsBtn.addEventListener("click", () => {
      capturedGps = { lat: 31.5204, lng: 74.3587 };
      useGpsDenied = false;
      openPodModal(container, del);
    });
  }

  const overrideGpsBtn = modalContainer.querySelector("#btn-override-gps");
  if (overrideGpsBtn) {
    overrideGpsBtn.addEventListener("click", () => {
      useGpsDenied = true;
      capturedGps = null;
      openPodModal(container, del);
    });
  }

  const submitBtn = modalContainer.querySelector("#btn-submit-pod");
  const errorBox = modalContainer.querySelector("#pod-error-msg");

  if (submitBtn && errorBox) {
    submitBtn.addEventListener("click", () => {
      const nameInput = modalContainer.querySelector("#pod-name") as HTMLInputElement | null;
      const rxCheck = modalContainer.querySelector("#pod-rx-check") as HTMLInputElement | null;
      const cnicInput = modalContainer.querySelector("#pod-cnic") as HTMLInputElement | null;

      if (!capturedPhotoData) {
        errorBox.classList.remove("hidden");
        errorBox.textContent = t.photoMandatoryError;
        return;
      }

      if (!nameInput || !nameInput.value.trim()) {
        errorBox.classList.remove("hidden");
        errorBox.textContent = t.recipientNameRequired;
        return;
      }

      if (del.isControlled) {
        if (!rxCheck || !rxCheck.checked) {
          errorBox.classList.remove("hidden");
          errorBox.textContent = t.rxCollectedRequired;
          return;
        }
        if (!cnicInput || !/^\d{4}$/.test(cnicInput.value.trim())) {
          errorBox.classList.remove("hidden");
          errorBox.textContent = t.cnicMustBe4Digits;
          return;
        }
      }

      // Success POD
      del.status = "DELIVERED";
      capturedPhotoData = null;
      capturedGps = null;
      useGpsDenied = false;
      modalContainer.innerHTML = "";
      renderApp(container);
    });
  }
}

if (typeof document !== "undefined") {
  document.addEventListener("DOMContentLoaded", () => {
    initRiderApp();
  });
}
