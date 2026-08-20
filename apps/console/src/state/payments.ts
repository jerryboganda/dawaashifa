/**
 * Payment Proof Review State (Doc 16 §8, Invariant I-4)
 * Screenshot proof analysis, fraud flags sorted by severity, duplicate TID critical banner,
 * side-by-side total vs OCR comparison, and zero bulk approve controls.
 */

export type FraudSeverity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW";

export interface FraudFlagItem {
  code: string;
  severity: FraudSeverity;
  description: string;
  earlierOrderId?: string;
}

export interface PaymentReviewItem {
  id: string;
  orderId: string;
  customerName: string;
  orderTotalMoney: string; // e.g. "1250.0000"
  ocrExtractedMoney: string; // e.g. "1250.0000"
  screenshotUrl: string;
  transactionId: string;
  flags: FraudFlagItem[];
  decision: "APPROVED" | "REJECTED" | null;
  rejectionReason?: string;
}

export class PaymentReviewManager {
  public queue: PaymentReviewItem[] = [];
  public currentItemIndex: number = 0;

  constructor(queue: PaymentReviewItem[] = []) {
    this.queue = queue;
    this.sortQueueBySeverity();
  }

  public get currentItem(): PaymentReviewItem | undefined {
    return this.queue[this.currentItemIndex];
  }

  public sortQueueBySeverity(): void {
    const rank: Record<FraudSeverity, number> = {
      CRITICAL: 4,
      HIGH: 3,
      MEDIUM: 2,
      LOW: 1,
    };

    this.queue.sort((a, b) => {
      const maxA = a.flags.reduce((max, f) => Math.max(max, rank[f.severity]), 0);
      const maxB = b.flags.reduce((max, f) => Math.max(max, rank[f.severity]), 0);
      return maxB - maxA;
    });
  }

  public get duplicateTidFlag(): FraudFlagItem | undefined {
    return this.currentItem?.flags.find((f) => f.code === "DUPLICATE_TID");
  }

  public get hasAmountMismatch(): boolean {
    if (!this.currentItem) return false;
    return this.currentItem.orderTotalMoney !== this.currentItem.ocrExtractedMoney;
  }

  public approvePayment(paymentId: string): boolean {
    const item = this.queue.find((p) => p.id === paymentId);
    if (!item) return false;
    item.decision = "APPROVED";
    return true;
  }

  public rejectPayment(paymentId: string, reason: string): boolean {
    const item = this.queue.find((p) => p.id === paymentId);
    if (!item) return false;
    item.decision = "REJECTED";
    item.rejectionReason = reason;
    return true;
  }
}
