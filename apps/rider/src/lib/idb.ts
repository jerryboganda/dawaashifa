export interface OfflineAction {
  id: string;
  type: "DELIVER" | "FAIL" | "DECLARE_CASH";
  targetId: string;
  payload: Record<string, unknown>;
  idempotencyKey: string;
  createdAt: number;
  attempts: number;
}

export class OfflineSyncQueue {
  private storageKey = "shifa_rider_offline_queue";
  private memoryStore: OfflineAction[] = [];

  constructor() {
    this.loadFromStorage();
  }

  private loadFromStorage() {
    if (typeof localStorage !== "undefined") {
      try {
        const raw = localStorage.getItem(this.storageKey);
        if (raw) {
          this.memoryStore = JSON.parse(raw);
        }
      } catch {
        this.memoryStore = [];
      }
    }
  }

  private saveToStorage() {
    if (typeof localStorage !== "undefined") {
      try {
        localStorage.setItem(this.storageKey, JSON.stringify(this.memoryStore));
      } catch {
        // storage full or disabled
      }
    }
  }

  public enqueue(
    type: "DELIVER" | "FAIL" | "DECLARE_CASH",
    targetId: string,
    payload: Record<string, unknown>,
    idempotencyKey?: string,
  ): OfflineAction {
    const action: OfflineAction = {
      id: `act_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`,
      type,
      targetId,
      payload,
      idempotencyKey:
        idempotencyKey || `idem_${Date.now()}_${Math.random().toString(36).substring(2, 9)}`,
      createdAt: Date.now(),
      attempts: 0,
    };

    this.memoryStore.push(action);
    this.saveToStorage();
    return action;
  }

  public getPending(): OfflineAction[] {
    return [...this.memoryStore];
  }

  public getPendingCount(): number {
    return this.memoryStore.length;
  }

  public remove(id: string): void {
    this.memoryStore = this.memoryStore.filter((item) => item.id !== id);
    this.saveToStorage();
  }

  public clear(): void {
    this.memoryStore = [];
    this.saveToStorage();
  }

  public async syncAll(
    sender: (action: OfflineAction) => Promise<boolean>,
  ): Promise<{ synced: number; failed: number }> {
    let synced = 0;
    let failed = 0;
    const items = [...this.memoryStore];

    for (const item of items) {
      try {
        item.attempts += 1;
        const success = await sender(item);
        if (success) {
          this.remove(item.id);
          synced += 1;
        } else {
          failed += 1;
        }
      } catch {
        failed += 1;
      }
    }

    this.saveToStorage();
    return { synced, failed };
  }
}

export const offlineQueue = new OfflineSyncQueue();
