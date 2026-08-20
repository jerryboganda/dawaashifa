/**
 * B2B Hospital & Implant Desk State (Doc 16 §9, Doc 14)
 * Quotations, AR Aging buckets, consignment, and device recall queries.
 */

export interface B2bAccountSummary {
  id: string;
  name: string;
  creditLimit: string;
  currentBalance: string;
  onHold: boolean;
  overdue90DaysMoney: string;
}

export interface RecallSearchResult {
  productId: string;
  batchId: string;
  serialNumber: string;
  status: "IN_WAREHOUSE" | "CONSIGNMENT" | "IMPLANTED";
  location: string;
  patientReference?: string;
}

export class B2bDeskManager {
  public accounts: B2bAccountSummary[] = [];

  constructor(accounts: B2bAccountSummary[] = []) {
    this.accounts = accounts;
  }

  public get accountsOnHold(): B2bAccountSummary[] {
    return this.accounts.filter((a) => a.onHold);
  }

  public get accountsWith90DayOverdue(): B2bAccountSummary[] {
    return this.accounts.filter((a) => a.overdue90DaysMoney !== "0.0000" && a.overdue90DaysMoney !== "0");
  }
}
