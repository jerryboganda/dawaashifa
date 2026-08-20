/**
 * Inventory & Expiry Risk State (Doc 16 §9)
 * Stock levels by branch, 90/60/30 day expiry risk dashboard, cold chain monitoring.
 */

export interface ExpiryRiskItem {
  productId: string;
  productName: string;
  batchNumber: string;
  expiryDate: string; // YYYY-MM-DD
  daysRemaining: number;
  quantity: number;
  valueAtRiskMoney: string;
}

export interface ColdChainLog {
  sensorId: string;
  branchName: string;
  currentTempCelsius: number;
  minAllowedCelsius: number;
  maxAllowedCelsius: number;
  isExcursion: boolean;
  timestamp: string;
}

export class InventoryManager {
  public expiryItems: ExpiryRiskItem[] = [];
  public coldChainLogs: ColdChainLog[] = [];

  constructor(expiryItems: ExpiryRiskItem[] = [], coldChainLogs: ColdChainLog[] = []) {
    this.expiryItems = expiryItems;
    this.coldChainLogs = coldChainLogs;
  }

  public get critical30Days(): ExpiryRiskItem[] {
    return this.expiryItems.filter((i) => i.daysRemaining <= 30);
  }

  public get warning60Days(): ExpiryRiskItem[] {
    return this.expiryItems.filter((i) => i.daysRemaining > 30 && i.daysRemaining <= 60);
  }

  public get alert90Days(): ExpiryRiskItem[] {
    return this.expiryItems.filter((i) => i.daysRemaining > 60 && i.daysRemaining <= 90);
  }

  public get activeExcursions(): ColdChainLog[] {
    return this.coldChainLogs.filter((l) => l.isExcursion);
  }
}
