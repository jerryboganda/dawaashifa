import type { paths } from "@shifa/shared";
import { offlineQueue } from "./idb";

export type DeliveryDto =
  paths["/api/v1/deliveries"]["get"]["responses"]["200"]["content"]["application/json"][0];
export type DeliverRequest =
  paths["/api/v1/deliveries/{id}/deliver"]["post"]["requestBody"]["content"]["application/json"];
export type FailDeliveryRequest =
  paths["/api/v1/deliveries/{id}/fail"]["post"]["requestBody"]["content"]["application/json"];
export type RiderDto =
  paths["/api/v1/riders"]["get"]["responses"]["200"]["content"]["application/json"][0];
export type RiderCashSessionDto =
  paths["/api/v1/cash-sessions"]["get"]["responses"]["200"]["content"]["application/json"][0];

export class RiderApiClient {
  private baseUrl: string;
  private token: string | null = null;

  constructor(baseUrl = "/api/v1") {
    this.baseUrl = baseUrl;
  }

  public setToken(token: string | null) {
    this.token = token;
  }

  private getHeaders(): HeadersInit {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (this.token) {
      headers["Authorization"] = `Bearer ${this.token}`;
    }
    return headers;
  }

  public async getTodayDeliveries(): Promise<DeliveryDto[]> {
    const res = await fetch(`${this.baseUrl}/deliveries`, {
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Failed to load deliveries: ${res.statusText}`);
    }
    return res.json();
  }

  public async acceptDelivery(deliveryId: string): Promise<DeliveryDto> {
    const res = await fetch(`${this.baseUrl}/deliveries/${deliveryId}/accept`, {
      method: "POST",
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Failed to accept delivery: ${res.statusText}`);
    }
    return res.json();
  }

  public async declineDelivery(deliveryId: string, reason: string): Promise<DeliveryDto> {
    const res = await fetch(`${this.baseUrl}/deliveries/${deliveryId}/decline`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify({ reason }),
    });
    if (!res.ok) {
      throw new Error(`Failed to decline delivery: ${res.statusText}`);
    }
    return res.json();
  }

  public async pickupDelivery(deliveryId: string): Promise<DeliveryDto> {
    const res = await fetch(`${this.baseUrl}/deliveries/${deliveryId}/pickup`, {
      method: "POST",
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Failed to mark delivery picked up: ${res.statusText}`);
    }
    return res.json();
  }

  public async completeDelivery(
    deliveryId: string,
    req: DeliverRequest,
  ): Promise<{ delivery?: DeliveryDto; queuedOffline?: boolean }> {
    const idempotencyKey =
      req.idempotency_key || `idem_pod_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
    const payload = { ...req, idempotency_key: idempotencyKey };

    try {
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        offlineQueue.enqueue("DELIVER", deliveryId, payload, idempotencyKey);
        return { queuedOffline: true };
      }

      const res = await fetch(`${this.baseUrl}/deliveries/${deliveryId}/deliver`, {
        method: "POST",
        headers: this.getHeaders(),
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        // If network failure / 5xx, save to offline queue
        if (res.status >= 500 || res.status === 0) {
          offlineQueue.enqueue("DELIVER", deliveryId, payload, idempotencyKey);
          return { queuedOffline: true };
        }
        const errJson = await res.json().catch(() => ({}));
        throw new Error(errJson.error?.message || `Delivery completion failed: ${res.statusText}`);
      }

      const delivery = await res.json();
      return { delivery };
    } catch (err: unknown) {
      offlineQueue.enqueue("DELIVER", deliveryId, payload, idempotencyKey);
      return { queuedOffline: true };
    }
  }

  public async failDelivery(
    deliveryId: string,
    req: FailDeliveryRequest,
  ): Promise<{ delivery?: DeliveryDto; queuedOffline?: boolean }> {
    const idempotencyKey =
      req.idempotency_key || `idem_fail_${Date.now()}_${Math.random().toString(36).substring(2, 8)}`;
    const payload = { ...req, idempotency_key: idempotencyKey };

    try {
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        offlineQueue.enqueue("FAIL", deliveryId, payload, idempotencyKey);
        return { queuedOffline: true };
      }

      const res = await fetch(`${this.baseUrl}/deliveries/${deliveryId}/fail`, {
        method: "POST",
        headers: this.getHeaders(),
        body: JSON.stringify(payload),
      });

      if (!res.ok) {
        if (res.status >= 500 || res.status === 0) {
          offlineQueue.enqueue("FAIL", deliveryId, payload, idempotencyKey);
          return { queuedOffline: true };
        }
        const errJson = await res.json().catch(() => ({}));
        throw new Error(errJson.error?.message || `Failure report failed: ${res.statusText}`);
      }

      const delivery = await res.json();
      return { delivery };
    } catch {
      offlineQueue.enqueue("FAIL", deliveryId, payload, idempotencyKey);
      return { queuedOffline: true };
    }
  }

  public async getCashSessions(): Promise<RiderCashSessionDto[]> {
    const res = await fetch(`${this.baseUrl}/cash-sessions`, {
      headers: this.getHeaders(),
    });
    if (!res.ok) {
      throw new Error(`Failed to load cash sessions: ${res.statusText}`);
    }
    return res.json();
  }

  public async declareCash(
    sessionId: string,
    collectedAmount: string,
  ): Promise<RiderCashSessionDto> {
    const res = await fetch(`${this.baseUrl}/cash-sessions/${sessionId}/declare`, {
      method: "POST",
      headers: this.getHeaders(),
      body: JSON.stringify({ collected_amount: collectedAmount }),
    });
    if (!res.ok) {
      throw new Error(`Failed to submit cash declaration: ${res.statusText}`);
    }
    return res.json();
  }

  public async syncOfflineQueue(): Promise<{ synced: number; failed: number }> {
    return offlineQueue.syncAll(async (action) => {
      try {
        if (action.type === "DELIVER") {
          const res = await fetch(`${this.baseUrl}/deliveries/${action.targetId}/deliver`, {
            method: "POST",
            headers: this.getHeaders(),
            body: JSON.stringify(action.payload),
          });
          return res.ok;
        } else if (action.type === "FAIL") {
          const res = await fetch(`${this.baseUrl}/deliveries/${action.targetId}/fail`, {
            method: "POST",
            headers: this.getHeaders(),
            body: JSON.stringify(action.payload),
          });
          return res.ok;
        } else if (action.type === "DECLARE_CASH") {
          const res = await fetch(`${this.baseUrl}/cash-sessions/${action.targetId}/declare`, {
            method: "POST",
            headers: this.getHeaders(),
            body: JSON.stringify(action.payload),
          });
          return res.ok;
        }
        return false;
      } catch {
        return false;
      }
    });
  }
}

export const riderApi = new RiderApiClient();
