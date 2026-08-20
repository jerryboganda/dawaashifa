/**
 * Prescription Review Queue State (Doc 16 §7, Invariant I-3)
 * Split-view image & extraction, per-line decisions, zero bulk approve,
 * approval disabled until all lines decided, and keyboard shortcuts.
 */

export interface RxLineCandidate {
  productId: string;
  name: string;
  inStock: boolean;
  score: number;
}

export interface RxExtractedLine {
  id: string;
  lineNumber: number;
  rawOcrText: string;
  matchedProduct: {
    productId: string;
    name: string;
    isControlled: boolean;
    confidence: number;
  };
  quantity: number;
  dosage: string;
  alternativeCandidates: RxLineCandidate[];
  decision: "ACCEPTED" | "EDITED" | "SUBSTITUTED" | "REJECTED" | null;
  substitutionReason?: string;
}

export interface PrescriptionReviewItem {
  id: string;
  patientName: string;
  createdAt: string; // ISO UTC
  imageUrl: string;
  lines: RxExtractedLine[];
  imageTransform: {
    zoom: number;
    rotation: number;
    contrast: boolean;
  };
}

export class RxReviewManager {
  public queue: PrescriptionReviewItem[] = [];
  public currentItemIndex: number = 0;
  public selectedLineNumber: number = 1;

  constructor(queue: PrescriptionReviewItem[] = []) {
    this.queue = queue;
  }

  public get currentItem(): PrescriptionReviewItem | undefined {
    return this.queue[this.currentItemIndex];
  }

  public get queueDepth(): number {
    return this.queue.length;
  }

  public get oldestWaiting(): string | null {
    if (this.queue.length === 0) return null;
    return this.queue[0].createdAt;
  }

  public get undecidLineCount(): number {
    if (!this.currentItem) return 0;
    return this.currentItem.lines.filter((l) => l.decision === null).length;
  }

  // Invariant I-3 & Doc 16 §7: Approve button disabled until 100% of lines have explicit decision
  public canApproveCurrentPrescription(): boolean {
    if (!this.currentItem || this.currentItem.lines.length === 0) return false;
    return this.undecidLineCount === 0;
  }

  public get hasControlledSubstance(): boolean {
    if (!this.currentItem) return false;
    return this.currentItem.lines.some((l) => l.matchedProduct.isControlled);
  }

  // Line actions (Accept, Edit, Substitute, Reject)
  public acceptLine(lineNumber: number): void {
    const line = this.currentItem?.lines.find((l) => l.lineNumber === lineNumber);
    if (line) line.decision = "ACCEPTED";
  }

  public editLine(lineNumber: number, newQty: number, newDosage: string): void {
    const line = this.currentItem?.lines.find((l) => l.lineNumber === lineNumber);
    if (line) {
      line.quantity = newQty;
      line.dosage = newDosage;
      line.decision = "EDITED";
    }
  }

  public substituteLine(lineNumber: number, candidate: RxLineCandidate, reason: string): void {
    const line = this.currentItem?.lines.find((l) => l.lineNumber === lineNumber);
    if (line) {
      line.matchedProduct.productId = candidate.productId;
      line.matchedProduct.name = candidate.name;
      line.substitutionReason = reason;
      line.decision = "SUBSTITUTED";
    }
  }

  public rejectLine(lineNumber: number): void {
    const line = this.currentItem?.lines.find((l) => l.lineNumber === lineNumber);
    if (line) line.decision = "REJECTED";
  }

  // Keyboard shortcut execution (Doc 16 §7)
  public handleKeyboardShortcut(key: string, ctrl: boolean): boolean {
    if (!this.currentItem) return false;

    // Numbers 1-9 select line
    const num = parseInt(key, 10);
    if (!isNaN(num) && num >= 1 && num <= this.currentItem.lines.length) {
      this.selectedLineNumber = num;
      return true;
    }

    if (key === "a" || key === "A") {
      this.acceptLine(this.selectedLineNumber);
      return true;
    }

    if (key === "x" || key === "X") {
      this.rejectLine(this.selectedLineNumber);
      return true;
    }

    if (ctrl && key === "Enter") {
      if (this.canApproveCurrentPrescription()) {
        this.submitApproval();
        return true;
      }
    }

    return false;
  }

  public submitApproval(): boolean {
    if (!this.canApproveCurrentPrescription()) {
      return false; // Blocked!
    }
    // Advance queue
    this.queue.splice(this.currentItemIndex, 1);
    this.selectedLineNumber = 1;
    return true;
  }
}
