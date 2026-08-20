export type Language = "en" | "ur" | "roman_ur";

export interface TranslationDict {
  appTitle: string;
  onShift: string;
  offShift: string;
  startShift: string;
  endShift: string;
  offline: string;
  online: string;
  syncPending: string;
  syncSuccess: string;
  todayDeliveries: string;
  completedToday: string;
  expectedCash: string;
  deliveriesTab: string;
  cashTab: string;
  historyTab: string;
  orderRef: string;
  status: string;
  codAmount: string;
  prepaid: string;
  items: string;
  callCustomer: string;
  whatsappCustomer: string;
  navigateAddress: string;
  accept: string;
  decline: string;
  pickedUp: string;
  startDelivery: string;
  deliverPod: string;
  markFailed: string;
  podTitle: string;
  takePhoto: string;
  photoCaptured: string;
  photoMandatoryError: string;
  recipientName: string;
  recipientNameRequired: string;
  recipientCnicLast4: string;
  cnicMustBe4Digits: string;
  controlledSubstanceWarning: string;
  originalRxCollected: string;
  rxCollectedRequired: string;
  cashCollectedPrompt: string;
  captureGps: string;
  gpsCaptured: string;
  gpsDeniedWarning: string;
  useGpsDeniedOverride: string;
  signatureOptional: string;
  clearSignature: string;
  submitPod: string;
  failedTitle: string;
  failReason: string;
  reasonUnreachable: string;
  reasonRefused: string;
  reasonWrongAddress: string;
  reasonReschedule: string;
  reasonOther: string;
  submitFailure: string;
  cashReconTitle: string;
  cashExpectedToday: string;
  declareCollectedAmount: string;
  submitDeclaration: string;
  cashierDeposited: string;
  variance: string;
  varianceNote: string;
  reconciled: string;
  unreconciled: string;
}

export const translations: Record<Language, TranslationDict> = {
  en: {
    appTitle: "Shifa Rider",
    onShift: "ON SHIFT",
    offShift: "OFF SHIFT",
    startShift: "Go On Shift",
    endShift: "End Shift",
    offline: "Offline Mode (Queued)",
    online: "Online",
    syncPending: "sync items pending",
    syncSuccess: "All synced",
    todayDeliveries: "Today's Deliveries",
    completedToday: "Completed",
    expectedCash: "COD Expected",
    deliveriesTab: "Deliveries",
    cashTab: "Cash Recon",
    historyTab: "History",
    orderRef: "Order Ref",
    status: "Status",
    codAmount: "COD Cash to Collect",
    prepaid: "Prepaid (No Cash)",
    items: "Order Items",
    callCustomer: "Call Customer",
    whatsappCustomer: "WhatsApp",
    navigateAddress: "Navigate",
    accept: "Accept Delivery",
    decline: "Decline",
    pickedUp: "Picked Up from Branch",
    startDelivery: "Start Delivery (In Transit)",
    deliverPod: "Complete Delivery (POD)",
    markFailed: "Mark as Failed",
    podTitle: "Proof of Delivery",
    takePhoto: "Capture Parcel Photo *",
    photoCaptured: "Photo Captured",
    photoMandatoryError: "Parcel photo is mandatory",
    recipientName: "Recipient Full Name *",
    recipientNameRequired: "Recipient name is required",
    recipientCnicLast4: "Recipient CNIC Last 4 Digits *",
    cnicMustBe4Digits: "CNIC last 4 digits must be exactly 4 numbers",
    controlledSubstanceWarning: "Controlled Substance: Physical prescription and CNIC last 4 digits required by DRAP law.",
    originalRxCollected: "I have physically collected the doctor's original prescription *",
    rxCollectedRequired: "You must collect the original prescription",
    cashCollectedPrompt: "Cash Collected (Rs)",
    captureGps: "Capture GPS Location",
    gpsCaptured: "GPS Location Attached",
    gpsDeniedWarning: "GPS location is recommended for audit verification.",
    useGpsDeniedOverride: "Proceed without GPS (Mark GPS Denied)",
    signatureOptional: "Customer Signature (Optional)",
    clearSignature: "Clear Signature",
    submitPod: "Confirm & Complete Delivery",
    failedTitle: "Report Delivery Failure",
    failReason: "Failure Reason",
    reasonUnreachable: "Customer phone unreachable / switched off",
    reasonRefused: "Customer refused parcel",
    reasonWrongAddress: "Incorrect or incomplete address",
    reasonReschedule: "Customer requested reschedule",
    reasonOther: "Other",
    submitFailure: "Submit Failure Report",
    cashReconTitle: "Daily Cash Reconciliation",
    cashExpectedToday: "Total COD Cash Expected",
    declareCollectedAmount: "Declare Collected Cash (Rs)",
    submitDeclaration: "Submit Shift Declaration",
    cashierDeposited: "Deposited to Cashier",
    variance: "Variance",
    varianceNote: "Variance Reason Note",
    reconciled: "Reconciled & Closed",
    unreconciled: "Open / Pending Cashier",
  },
  ur: {
    appTitle: "شفا رائڈر",
    onShift: "ڈیوٹی پر",
    offShift: "ڈیوٹی بند",
    startShift: "ڈیوٹی شروع کریں",
    endShift: "ڈیوٹی ختم کریں",
    offline: "آف لائن موڈ (محفوظ ہو رہا ہے)",
    online: "آن لائن",
    syncPending: "بقایا ڈیلیوری سنک ہو رہی ہے",
    syncSuccess: "تمام ڈیٹا ہم آہنگ ہے",
    todayDeliveries: "آج کی ڈیلیوریز",
    completedToday: "مکمل شدہ",
    expectedCash: "متوقع کیش",
    deliveriesTab: "ڈیلیوریز",
    cashTab: "کیش حساب",
    historyTab: "ریکارڈ",
    orderRef: "آرڈر نمبر",
    status: "حالت",
    codAmount: "وصول کرنے والی کیش رقم",
    prepaid: "پہلے سے ادا شدہ (کیش نہیں)",
    items: "ادویات کی فہرست",
    callCustomer: "فون کال کریں",
    whatsappCustomer: "واٹس ایپ",
    navigateAddress: "راستہ دیکھیں (میپ)",
    accept: "آرڈر قبول کریں",
    decline: "مسترد کریں",
    pickedUp: "برانچ سے اٹھا لیا",
    startDelivery: "ڈیلیوری کے لیے روانہ",
    deliverPod: "ڈیلیوری مکمل ثبوت (POD)",
    markFailed: "ڈیلیوری ناکام درج کریں",
    podTitle: "ڈیلیوری کی تصدیق",
    takePhoto: "پارسل کی تصویر لیں *",
    photoCaptured: "تصویر محفوظ ہو گئی",
    photoMandatoryError: "تصویر لینا لازمی ہے",
    recipientName: "وصول کنندہ کا نام *",
    recipientNameRequired: "وصول کنندہ کا نام درج کریں",
    recipientCnicLast4: "شناختی کارڈ کے آخری 4 ہندسے *",
    cnicMustBe4Digits: "شناختی کارڈ کے آخری 4 ہندسے درج کریں",
    controlledSubstanceWarning: "مخصوص دوا: اصل ڈاکٹر کا نسخہ اور شناختی کارڈ نمبر لینا قانونی طور پر لازمی ہے۔",
    originalRxCollected: "میں نے اصل نسخہ کسٹمر سے وصول کر لیا ہے *",
    rxCollectedRequired: "اصل نسخہ وصول کرنا لازمی ہے",
    cashCollectedPrompt: "وصول شدہ کیش رقم (روپے)",
    captureGps: "جی پی ایس لوکیشن محفوظ کریں",
    gpsCaptured: "لوکیشن محفوظ ہو گئی",
    gpsDeniedWarning: "لوکیشن کی تصدیق ضروری ہے۔",
    useGpsDeniedOverride: "بغیر لوکیشن کے جاری رکھیں",
    signatureOptional: "گاہک کے دستخط (اختیاری)",
    clearSignature: "دستخط مٹائیں",
    submitPod: "ڈیلیوری تصدیق کر کے مکمل کریں",
    failedTitle: "ڈیلیوری ناکامی کی رپورٹ",
    failReason: "ناکامی کی وجہ",
    reasonUnreachable: "کسٹمر کا فون بند یا نہیں ملا",
    reasonRefused: "کسٹمر نے پارسل لینے سے انکار کر دیا",
    reasonWrongAddress: "پتہ غلط یا نامکمل ہے",
    reasonReschedule: "کسٹمر نے وقت تبدیل کرنے کا کہا",
    reasonOther: "دیگر وجہ",
    submitFailure: "ناکامی کی رپورٹ جمع کریں",
    cashReconTitle: "یومیہ کیش کا حساب",
    cashExpectedToday: "آج کا کل متوقع کیش",
    declareCollectedAmount: "جمع شدہ کیش رقم درج کریں (روپے)",
    submitDeclaration: "کیش کا حساب جمع کروائیں",
    cashierDeposited: "کیشیئر کو جمع کروائی گئی رقم",
    variance: "فرق (کمی یا بیشی)",
    varianceNote: "فرق کی وجہ",
    reconciled: "حساب کلیئر اور بند",
    unreconciled: "کیشیئر کی تصدیق باقی",
  },
  roman_ur: {
    appTitle: "Shifa Rider",
    onShift: "ON SHIFT",
    offShift: "OFF SHIFT",
    startShift: "Duty Shuru Karein",
    endShift: "Duty Khatam Karein",
    offline: "Offline Mode (Queue mein hai)",
    online: "Online",
    syncPending: "Deliveries sync ho rahi hain",
    syncSuccess: "Sab sync ho gaya",
    todayDeliveries: "Aaj ki Deliveries",
    completedToday: "Mukammal",
    expectedCash: "COD Cash Expected",
    deliveriesTab: "Deliveries",
    cashTab: "Cash Hisaab",
    historyTab: "Record",
    orderRef: "Order Number",
    status: "Status",
    codAmount: "COD Cash Lena Hai",
    prepaid: "Prepaid (Cash nahi lena)",
    items: "Medicines List",
    callCustomer: "Call Karein",
    whatsappCustomer: "WhatsApp Karein",
    navigateAddress: "Map Pe Dekhein",
    accept: "Order Accept Karein",
    decline: "Decline Karein",
    pickedUp: "Branch se utha liya",
    startDelivery: "Raste mein hain (In Transit)",
    deliverPod: "Delivery Complete (POD)",
    markFailed: "Delivery Failed",
    podTitle: "Proof of Delivery",
    takePhoto: "Parcel ki Tasveer lein *",
    photoCaptured: "Tasveer save ho gayi",
    photoMandatoryError: "Tasveer lena lazmi hai",
    recipientName: "Receiver ka Naam *",
    recipientNameRequired: "Receiver ka naam lazmi hai",
    recipientCnicLast4: "Receiver CNIC ke aakhri 4 digits *",
    cnicMustBe4Digits: "CNIC ke aakhri 4 number likhein",
    controlledSubstanceWarning: "Controlled Dawa: Asal doctor ka prescription aur CNIC number lena qanoonan lazmi hai.",
    originalRxCollected: "Maine doctor ka asal prescription le liya hai *",
    rxCollectedRequired: "Asal prescription lena lazmi hai",
    cashCollectedPrompt: "Cash Wasool Huwa (Rs)",
    captureGps: "GPS Location lein",
    gpsCaptured: "Location add ho gayi",
    gpsDeniedWarning: "GPS location verification zaroori hai.",
    useGpsDeniedOverride: "Baghair GPS jari rakhein",
    signatureOptional: "Customer Signature (Optional)",
    clearSignature: "Signature Hatayein",
    submitPod: "Delivery Mukammal Karein",
    failedTitle: "Delivery Failure Report",
    failReason: "Wajah",
    reasonUnreachable: "Customer ka phone band tha",
    reasonRefused: "Customer ne parcel lene se inkar kiya",
    reasonWrongAddress: "Address ghalat tha",
    reasonReschedule: "Customer ne baad mein delivery ka kaha",
    reasonOther: "Koi aur wajah",
    submitFailure: "Report Submit Karein",
    cashReconTitle: "Rozana Cash Ka Hisaab",
    cashExpectedToday: "Total COD Cash Expected",
    declareCollectedAmount: "Wasool kiya gaya Cash likhein (Rs)",
    submitDeclaration: "Declaration Submit Karein",
    cashierDeposited: "Cashier ko jama karwaya",
    variance: "Farq (Variance)",
    varianceNote: "Farq ki wajah",
    reconciled: "Reconciled ho gaya",
    unreconciled: "Pending Cashier",
  },
};

export function isRTL(lang: Language): boolean {
  return lang === "ur";
}
