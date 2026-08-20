/**
 * Multilingual internationalization catalogue (Doc 16 §10).
 * Three locales: en (English), ur (Urdu Script - RTL), ur-Latn (Roman Urdu - LTR).
 */

export type Locale = "en" | "ur" | "ur-Latn";

export interface TranslationCatalogue {
  dir: "ltr" | "rtl";
  common: {
    loading: string;
    empty: string;
    error: string;
    save: string;
    cancel: string;
    accept: string;
    reject: string;
    edit: string;
    substitute: string;
    discard: string;
    send: string;
    reconnect: string;
    status: string;
    search: string;
    branch: string;
  };
  inbox: {
    title: string;
    draftBadge: string;
    audioTranscript: string;
    sendDraft: string;
    editDraft: string;
    discardDraft: string;
    rxExclusionNotice: string;
  };
  rxReview: {
    title: string;
    queueBacklog: string;
    oldestWaiting: string;
    controlledWarning: string;
    ocrRaw: string;
    matchedProduct: string;
    confidence: string;
    alternatives: string;
    approvePrescription: string;
    allLinesRequired: string;
    linesRemaining: string;
  };
  payments: {
    title: string;
    fraudFlags: string;
    duplicateTidWarning: string;
    proofImage: string;
    ocrMismatch: string;
    approvePayment: string;
    rejectPayment: string;
  };
  orders: {
    title: string;
    illegalDropBlocked: string;
    timeline: string;
  };
  b2b: {
    title: string;
    quotes: string;
    accounts: string;
    arAging: string;
    recallLookup: string;
    consignment: string;
  };
}

export const translations: Record<Locale, TranslationCatalogue> = {
  en: {
    dir: "ltr",
    common: {
      loading: "Loading data...",
      empty: "No records found.",
      error: "An unexpected error occurred. Please retry.",
      save: "Save Changes",
      cancel: "Cancel",
      accept: "Accept",
      reject: "Reject",
      edit: "Edit",
      substitute: "Substitute",
      discard: "Discard",
      send: "Send",
      reconnect: "Reconnecting to live event stream...",
      status: "Status",
      search: "Search...",
      branch: "Branch",
    },
    inbox: {
      title: "Unified WhatsApp Inbox",
      draftBadge: "AI Draft",
      audioTranscript: "Voice Transcript",
      sendDraft: "Send Draft",
      editDraft: "Edit Draft",
      discardDraft: "Discard",
      rxExclusionNotice: "Rx-linked conversations are excluded from bulk sending (Invariant I-6)",
    },
    rxReview: {
      title: "Prescription Review Queue",
      queueBacklog: "Queue Backlog",
      oldestWaiting: "Oldest Waiting",
      controlledWarning: "CONTROLLED SUBSTANCE: Special handling and identity verification required",
      ocrRaw: "Raw OCR Text",
      matchedProduct: "Matched Medicine",
      confidence: "Confidence",
      alternatives: "Alternatives",
      approvePrescription: "Approve Prescription",
      allLinesRequired: "All prescription lines must have an explicit decision before approval",
      linesRemaining: "decisions remaining",
    },
    payments: {
      title: "Payment Proof Review",
      fraudFlags: "Fraud & Validation Flags",
      duplicateTidWarning: "CRITICAL: DUPLICATE TRANSACTION ID DETECTED",
      proofImage: "Payment Proof Screenshot",
      ocrMismatch: "Discrepancy between Proof Amount and Order Total",
      approvePayment: "Approve Payment",
      rejectPayment: "Reject Payment",
    },
    orders: {
      title: "Order Fulfillment Kanban",
      illegalDropBlocked: "Illegal order status transition rejected by state machine",
      timeline: "Order Event Timeline",
    },
    b2b: {
      title: "B2B Medical Device Desk",
      quotes: "Quotations",
      accounts: "Hospital Accounts",
      arAging: "AR Aging & Credit",
      recallLookup: "Device Recall Inquiry",
      consignment: "Consignment Inventory",
    },
  },
  ur: {
    dir: "rtl",
    common: {
      loading: "معلومات لوڈ ہو رہی ہے...",
      empty: "کوئی ریکارڈ موجود نہیں ہے۔",
      error: "ایک غیر متوقع خرابی پیش آگئی ہے۔ برائے مہربانی دوبارہ کوشش کریں۔",
      save: "محفوظ کریں",
      cancel: "منسوخ کریں",
      accept: "منظور",
      reject: "مسترد",
      edit: "ترمیم کریں",
      substitute: "متبادل دوا",
      discard: "خارج کریں",
      send: "ارسال کریں",
      reconnect: "دوبارہ رابطہ کیا جا رہا ہے...",
      status: "حالت",
      search: "تلاش کریں...",
      branch: "برانچ",
    },
    inbox: {
      title: "واٹس ایپ ان باکس",
      draftBadge: "مصنوعی ذہانت مسودہ",
      audioTranscript: "صوتی پیغام کی تحریر",
      sendDraft: "مسودہ بھیجیں",
      editDraft: "ترمیم کریں",
      discardDraft: "رد کریں",
      rxExclusionNotice: "نسخے والے پیغامات کو اجتماعی ترسیل سے خارج رکھا گیا ہے",
    },
    rxReview: {
      title: "نسخہ جات کی تصدیق",
      queueBacklog: "منتظر نسخے",
      oldestWaiting: "سب سے پرانا انتظار",
      controlledWarning: "تنبیہ: یہ نشہ آور/ممنوعہ دوا ہے۔ خصوصی تصدیق لازمی ہے",
      ocrRaw: "اصل تحریر",
      matchedProduct: "منتخب دوا",
      confidence: "درستگی",
      alternatives: "متبادل ادویات",
      approvePrescription: "نسخہ منظور کریں",
      allLinesRequired: "تمام ادویات پر حتمی فیصلہ لازمی ہے",
      linesRemaining: "فیصلے باقی ہیں",
    },
    payments: {
      title: "ادائیگی کی تصدیق",
      fraudFlags: "تصدیقی تنبیہات",
      duplicateTidWarning: "خطرہ: یہ ٹرانزیکشن شناختی نمبر پہلے استعمال ہو چکا ہے",
      proofImage: "رسید کی تصویر",
      ocrMismatch: "رسید کی رقم اور آرڈر کے بل میں فرق ہے",
      approvePayment: "ادائیگی منظور کریں",
      rejectPayment: "ادائیگی مسترد کریں",
    },
    orders: {
      title: "آرڈرز بورڈ",
      illegalDropBlocked: "غیر مجاز آرڈر تبدیلی مسترد کر دی گئی",
      timeline: "آرڈر کی تفصیلات و تاریخ",
    },
    b2b: {
      title: "ہسپتال اور امپلانٹس ڈیسک",
      quotes: "کوٹیشنز",
      accounts: "ہسپتال کھاتے",
      arAging: "ادائیگی کے بقایا جات",
      recallLookup: "امپلانٹس واپسی تلاش",
      consignment: "کنسائنمنٹ اسٹاک",
    },
  },
  "ur-Latn": {
    dir: "ltr",
    common: {
      loading: "Data load ho raha hai...",
      empty: "Koi record nahi mila.",
      error: "Ghalti pesh aayi hai. Dobara koshish karein.",
      save: "Save karein",
      cancel: "Cancel",
      accept: "Manzoor",
      reject: "Radd",
      edit: "Tabdeeli",
      substitute: "Mutabadil Dawa",
      discard: "Khatam karein",
      send: "Bheinjein",
      reconnect: "Dobara connect ho raha hai...",
      status: "Status",
      search: "Talash karein...",
      branch: "Branch",
    },
    inbox: {
      title: "WhatsApp Inbox",
      draftBadge: "AI Draft",
      audioTranscript: "Voice Note Transcript",
      sendDraft: "Draft Bheinjein",
      editDraft: "Draft Edit Karein",
      discardDraft: "Discard Karein",
      rxExclusionNotice: "Nuskhe walay orders bulk message se baahir hain",
    },
    rxReview: {
      title: "Nuskha Tashkhees Desk",
      queueBacklog: "Intezar Queue",
      oldestWaiting: "Purana Intezar",
      controlledWarning: "KHAS TANBEEH: Controlled dawa! Tasdeeq zaroori hai",
      ocrRaw: "Asal Likhaai",
      matchedProduct: "Mili Hui Dawa",
      confidence: "Yaqeen",
      alternatives: "Doosray Options",
      approvePrescription: "Nuskha Approve Karein",
      allLinesRequired: "Har dawa ka faisla zaroori hai",
      linesRemaining: "faislay baqi hain",
    },
    payments: {
      title: "Payment Receipt Review",
      fraudFlags: "Validation Flags",
      duplicateTidWarning: "KHATRA: YE TRANSACTION ID PEHLAY ISTEMAAL HO CHUKI HAI",
      proofImage: "Screenshot Image",
      ocrMismatch: "Screenshot aur bill ki raqam mein farq hai",
      approvePayment: "Payment Manzoor Karein",
      rejectPayment: "Payment Radd Karein",
    },
    orders: {
      title: "Order Board",
      illegalDropBlocked: "Ghalat order step reject ho gaya",
      timeline: "Order Timeline",
    },
    b2b: {
      title: "Hospital & Implants B2B",
      quotes: "Quotations",
      accounts: "Hospital Accounts",
      arAging: "Baqaya Jaat (Aging)",
      recallLookup: "Device Recall Search",
      consignment: "Consignment Stock",
    },
  },
};
