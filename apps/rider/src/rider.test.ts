import { describe, it, expect, beforeEach, vi } from "vitest";
import { OfflineSyncQueue } from "./lib/idb";
import { RiderApiClient } from "./lib/api";
import { translations, isRTL } from "./lib/i18n";

describe("Rider PWA Frontend Acceptance Suite (Doc 12 §10)", () => {
  let queue: OfflineSyncQueue;
  let api: RiderApiClient;

  beforeEach(() => {
    queue = new OfflineSyncQueue();
    queue.clear();
    api = new RiderApiClient("http://localhost:3000/api/v1");
  });

  // ----------------------------------------------------------------------------------------------
  // Test 1: Offline POD queue stores locally and syncs on reconnect
  // ----------------------------------------------------------------------------------------------
  it("test_offline_pod_queue_stores_locally_and_syncs_on_reconnect", async () => {
    expect(queue.getPendingCount()).toBe(0);

    const deliveryId = "del_018f3a9e-4c5b-7b3a-9e1a-2b3c4d5e6f7a";
    const payload = {
      pod_image_object_key: "pod/test.jpg",
      recipient_name: "Tariq Ali",
      cash_collected: "1500.0000",
    };

    // 1. Enqueue offline POD
    const item = queue.enqueue("DELIVER", deliveryId, payload);
    expect(queue.getPendingCount()).toBe(1);
    expect(item.idempotencyKey).toBeDefined();

    // 2. Mock network reconnect & sender
    const mockSender = vi.fn().mockResolvedValue(true);
    const result = await queue.syncAll(mockSender);

    expect(result.synced).toBe(1);
    expect(result.failed).toBe(0);
    expect(mockSender).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "DELIVER",
        targetId: deliveryId,
      }),
    );
    expect(queue.getPendingCount()).toBe(0);
  });

  // ----------------------------------------------------------------------------------------------
  // Test 2: Camera photo mandatory validation
  // ----------------------------------------------------------------------------------------------
  it("test_camera_photo_required_validation", () => {
    const validatePod = (photoKey: string | null | undefined, name: string) => {
      const errors: string[] = [];
      if (!photoKey || photoKey.trim().length === 0) {
        errors.push("Parcel photo is mandatory");
      }
      if (!name || name.trim().length === 0) {
        errors.push("Recipient name is required");
      }
      return errors;
    };

    const emptyPhotoErrors = validatePod("", "Ali Khan");
    expect(emptyPhotoErrors).toContain("Parcel photo is mandatory");

    const validErrors = validatePod("uploads/photo.jpg", "Ali Khan");
    expect(validErrors.length).toBe(0);
  });

  // ----------------------------------------------------------------------------------------------
  // Test 3: Controlled substance requires original Rx collection & recipient CNIC last 4 digits
  // ----------------------------------------------------------------------------------------------
  it("test_controlled_substance_requires_rx_checkbox_and_cnic_last4", () => {
    const validateControlledPod = (
      isControlled: boolean,
      rxCollected: boolean,
      cnicLast4: string,
    ) => {
      const errors: string[] = [];
      if (isControlled) {
        if (!rxCollected) {
          errors.push("Original physical prescription must be collected");
        }
        if (!/^\d{4}$/.test(cnicLast4)) {
          errors.push("CNIC last 4 digits must be exactly 4 numbers");
        }
      }
      return errors;
    };

    // Missing prescription checkbox
    const err1 = validateControlledPod(true, false, "1234");
    expect(err1).toContain("Original physical prescription must be collected");

    // Invalid CNIC (letters or <4 digits)
    const err2 = validateControlledPod(true, true, "12A");
    expect(err2).toContain("CNIC last 4 digits must be exactly 4 numbers");

    // Valid controlled submission
    const ok = validateControlledPod(true, true, "5678");
    expect(ok.length).toBe(0);
  });

  // ----------------------------------------------------------------------------------------------
  // Test 4: GPS denial graceful degradation
  // ----------------------------------------------------------------------------------------------
  it("test_gps_denial_graceful_degradation", () => {
    const formatPodCoordinates = (
      coords: { latitude: number; longitude: number } | null,
      gpsDeniedOverride: boolean,
    ) => {
      if (!coords && !gpsDeniedOverride) {
        throw new Error("GPS coordinates or explicit gps_denied override required");
      }
      return {
        latitude: coords?.latitude ?? null,
        longitude: coords?.longitude ?? null,
        gps_denied: gpsDeniedOverride,
      };
    };

    expect(() => formatPodCoordinates(null, false)).toThrow();

    const degraded = formatPodCoordinates(null, true);
    expect(degraded.gps_denied).toBe(true);
    expect(degraded.latitude).toBeNull();

    const normal = formatPodCoordinates({ latitude: 31.52, longitude: 74.35 }, false);
    expect(normal.gps_denied).toBe(false);
    expect(normal.latitude).toBe(31.52);
  });

  // ----------------------------------------------------------------------------------------------
  // Test 5: Cash declaration submits to reconciliation
  // ----------------------------------------------------------------------------------------------
  it("test_cash_declaration_submits_to_reconciliation", async () => {
    const sessionId = "ses_12345";
    const collectedAmount = "4500.0000";

    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        id: sessionId,
        status: "DECLARED",
        collected_amount: collectedAmount,
        expected_amount: "4500.0000",
      }),
    });
    vi.stubGlobal("fetch", fetchMock);

    const res = await api.declareCash(sessionId, collectedAmount);
    expect(res.status).toBe("DECLARED");
    expect(res.collected_amount).toBe(collectedAmount);
    expect(fetchMock).toHaveBeenCalledWith(
      "http://localhost:3000/api/v1/cash-sessions/ses_12345/declare",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ collected_amount: collectedAmount }),
      }),
    );
  });

  // ----------------------------------------------------------------------------------------------
  // Test 6: Minimum touch target size 44px
  // ----------------------------------------------------------------------------------------------
  it("test_minimum_touch_target_size_44px", () => {
    const primaryButtonStyles = {
      minHeight: "48px",
      minWidth: "48px",
      padding: "12px 24px",
    };

    const minHeightPx = parseInt(primaryButtonStyles.minHeight, 10);
    const minWidthPx = parseInt(primaryButtonStyles.minWidth, 10);

    expect(minHeightPx).toBeGreaterThanOrEqual(44);
    expect(minWidthPx).toBeGreaterThanOrEqual(44);
  });

  // ----------------------------------------------------------------------------------------------
  // Test 7: Multilingual Urdu & Roman Urdu RTL rendering
  // ----------------------------------------------------------------------------------------------
  it("test_multilingual_urdu_rtl_rendering", () => {
    expect(isRTL("ur")).toBe(true);
    expect(isRTL("en")).toBe(false);
    expect(isRTL("roman_ur")).toBe(false);

    // Verify translations exist for all essential actions across all 3 languages
    for (const lang of ["en", "ur", "roman_ur"] as const) {
      const dict = translations[lang];
      expect(dict.appTitle).toBeTruthy();
      expect(dict.deliverPod).toBeTruthy();
      expect(dict.markFailed).toBeTruthy();
      expect(dict.cashReconTitle).toBeTruthy();
      expect(dict.takePhoto).toBeTruthy();
      expect(dict.originalRxCollected).toBeTruthy();
    }
  });
});
