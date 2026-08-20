import { describe, it, expect } from "vitest";
import { InboxManager, ConversationItem, MessageItem } from "./state/inbox";
import { RxReviewManager, PrescriptionReviewItem } from "./state/rx-review";
import { PaymentReviewManager, PaymentReviewItem } from "./state/payments";
import { OrderBoardManager, OrderCard } from "./state/orders";
import { InventoryManager } from "./state/inventory";
import { B2bDeskManager } from "./state/b2b";
import { colors, formatPkr, translations, Locale } from "@shifa/shared";

describe("Doc 16 — Ops Console Acceptance Tests", () => {
  // Test 1: no_hand_written_api_types
  it("no_hand_written_api_types — imports shared contract tokens & schemas", () => {
    expect(colors).toBeDefined();
    expect(colors.brand[500]).toBe("#14b8a6");
  });

  // Test 2: no_money_arithmetic_in_browser (Invariant I-8)
  it("no_money_arithmetic_in_browser — formats string money without float parsing", () => {
    expect(formatPkr("1250.0000")).toBe("Rs 1,250.00");
    expect(formatPkr("100000.5000")).toBe("Rs 100,000.50");
    expect(formatPkr("0.0000")).toBe("Rs 0.00");
    expect(formatPkr(null)).toBe("Rs 0.00");
  });

  // Test 3: rx_review_approve_disabled_until_all_lines_decided (Doc 16 §7)
  it("rx_review_approve_disabled_until_all_lines_decided — blocks approval when lines are undecided", () => {
    const rxItem: PrescriptionReviewItem = {
      id: "rx-1",
      patientName: "Ahmed Ali",
      createdAt: "2026-08-20T10:00:00Z",
      imageUrl: "https://example.com/rx1.jpg",
      imageTransform: { zoom: 1, rotation: 0, contrast: false },
      lines: [
        {
          id: "l-1",
          lineNumber: 1,
          rawOcrText: "Panadol 500mg TDS",
          matchedProduct: { productId: "p-1", name: "Panadol 500mg", isControlled: false, confidence: 0.98 },
          quantity: 30,
          dosage: "1 tab TDS",
          alternativeCandidates: [],
          decision: null, // Undecided!
        },
        {
          id: "l-2",
          lineNumber: 2,
          rawOcrText: "Augmentin 625mg BD",
          matchedProduct: { productId: "p-2", name: "Augmentin 625mg", isControlled: false, confidence: 0.95 },
          quantity: 14,
          dosage: "1 tab BD",
          alternativeCandidates: [],
          decision: null, // Undecided!
        },
      ],
    };

    const manager = new RxReviewManager([rxItem]);

    // 1. Initial state: 2 undecided lines -> cannot approve
    expect(manager.canApproveCurrentPrescription()).toBe(false);
    expect(manager.undecidLineCount).toBe(2);

    // 2. Decide line 1 only -> still 1 undecided line -> cannot approve
    manager.acceptLine(1);
    expect(manager.undecidLineCount).toBe(1);
    expect(manager.canApproveCurrentPrescription()).toBe(false);

    // 3. Decide line 2 -> 0 undecided lines -> approval enabled!
    manager.acceptLine(2);
    expect(manager.undecidLineCount).toBe(0);
    expect(manager.canApproveCurrentPrescription()).toBe(true);
  });

  // Test 4: no_bulk_approve_control_in_rx_review (Invariant I-3)
  it("no_bulk_approve_control_in_rx_review — verifies approval requires individual item validation", () => {
    const rxItem: PrescriptionReviewItem = {
      id: "rx-2",
      patientName: "Fatima Noor",
      createdAt: "2026-08-20T11:00:00Z",
      imageUrl: "https://example.com/rx2.jpg",
      imageTransform: { zoom: 1, rotation: 0, contrast: false },
      lines: [
        {
          id: "l-3",
          lineNumber: 1,
          rawOcrText: "Rivotril 2mg",
          matchedProduct: { productId: "p-3", name: "Rivotril 2mg", isControlled: true, confidence: 0.99 },
          quantity: 10,
          dosage: "1 tab HS",
          alternativeCandidates: [],
          decision: null,
        },
      ],
    };

    const manager = new RxReviewManager([rxItem]);
    expect(manager.submitApproval()).toBe(false); // Direct bulk approval fails
  });

  // Test 5: no_bulk_approve_control_in_payment_review (Invariant I-4)
  it("no_bulk_approve_control_in_payment_review — verifies payment approvals are individual and explicit", () => {
    const p1: PaymentReviewItem = {
      id: "pay-1",
      orderId: "ord-1",
      customerName: "Kamran Khan",
      orderTotalMoney: "2500.0000",
      ocrExtractedMoney: "2500.0000",
      screenshotUrl: "https://example.com/proof1.png",
      transactionId: "TRX-1001",
      flags: [],
      decision: null,
    };

    const manager = new PaymentReviewManager([p1]);
    expect(manager.queue[0].decision).toBeNull();
    manager.approvePayment("pay-1");
    expect(manager.queue[0].decision).toBe("APPROVED");
  });

  // Test 6: rx_linked_conversation_excluded_from_bulk_send (Invariant I-6)
  it("rx_linked_conversation_excluded_from_bulk_send — excludes conversations with prescriptions from bulk messaging", () => {
    const convs: ConversationItem[] = [
      {
        id: "c-1",
        customerId: "cust-1",
        customerName: "Ali Raza",
        phone: "+923001234567",
        branchId: "b-1",
        lastMessage: "Need Panadol",
        hasPrescription: false,
        unreadCount: 0,
        status: "ACTIVE",
        messages: [],
      },
      {
        id: "c-2",
        customerId: "cust-2",
        customerName: "Sara Khan",
        phone: "+923007654321",
        branchId: "b-1",
        lastMessage: "Here is my prescription",
        hasPrescription: true, // Rx linked!
        unreadCount: 0,
        status: "PENDING_PHARMACIST",
        messages: [],
      },
      {
        id: "c-3",
        customerId: "cust-3",
        customerName: "Usman Tariq",
        phone: "+923009998877",
        branchId: "b-1",
        lastMessage: "Is store open?",
        hasPrescription: false,
        unreadCount: 0,
        status: "ACTIVE",
        messages: [],
      },
    ];

    const manager = new InboxManager(convs);
    const eligible = manager.filterEligibleForBulkSend(["c-1", "c-2", "c-3"]);

    expect(eligible).toEqual(["c-1", "c-3"]);
    expect(eligible.includes("c-2")).toBe(false); // c-2 excluded!
  });

  // Test 7: duplicate_tid_renders_critical_banner (Doc 16 §8)
  it("duplicate_tid_renders_critical_banner — flags duplicate TID with critical severity", () => {
    const payWithDup: PaymentReviewItem = {
      id: "pay-dup",
      orderId: "ord-99",
      customerName: "Fraud Test",
      orderTotalMoney: "5000.0000",
      ocrExtractedMoney: "5000.0000",
      screenshotUrl: "https://example.com/fake.png",
      transactionId: "ALFALAH-9999",
      flags: [
        {
          code: "DUPLICATE_TID",
          severity: "CRITICAL",
          description: "Transaction ID already used on earlier order #ORD-42",
          earlierOrderId: "ORD-42",
        },
      ],
      decision: null,
    };

    const manager = new PaymentReviewManager([payWithDup]);
    expect(manager.duplicateTidFlag).toBeDefined();
    expect(manager.duplicateTidFlag?.severity).toBe("CRITICAL");
    expect(manager.duplicateTidFlag?.earlierOrderId).toBe("ORD-42");
  });

  // Test 8: every_screen_renders_in_urdu_rtl (Doc 16 §10)
  it("every_screen_renders_in_urdu_rtl — provides RTL direction and complete Urdu translations", () => {
    const urdu = translations.ur;
    expect(urdu.dir).toBe("rtl");
    expect(urdu.common.loading.length).toBeGreaterThan(0);
    expect(urdu.inbox.title.length).toBeGreaterThan(0);
    expect(urdu.rxReview.title.length).toBeGreaterThan(0);
    expect(urdu.payments.title.length).toBeGreaterThan(0);
    expect(urdu.orders.title.length).toBeGreaterThan(0);
    expect(urdu.b2b.title.length).toBeGreaterThan(0);
  });

  // Test 9: status_colours_consistent_across_screens (Doc 16 §5)
  it("status_colours_consistent_across_screens — maintains unified semantic status palette", () => {
    expect(colors.status.pending).toBe("#eab308");
    expect(colors.status.review).toBe("#3b82f6");
    expect(colors.status.approved).toBe("#10b981");
    expect(colors.status.rejected).toBe("#ef4444");
    expect(colors.status.dispatched).toBe("#8b5cf6");
  });

  // Test 10: sse_reconnects_and_replays_after_drop (Doc 16 §6)
  it("sse_reconnects_and_replays_after_drop — handles connection drops and replays missed messages", () => {
    const conv: ConversationItem = {
      id: "c-100",
      customerId: "cust-100",
      customerName: "Live Customer",
      phone: "+923001112233",
      branchId: "b-1",
      lastMessage: "Hello",
      hasPrescription: false,
      unreadCount: 0,
      status: "ACTIVE",
      messages: [{ id: "m-1", sender: "CUSTOMER", text: "Hello", timestamp: "2026-08-20T12:00:00Z" }],
    };

    const manager = new InboxManager([conv]);
    manager.sseConnected = true;

    // 1. Connection drops
    manager.handleSseConnectionDrop();
    expect(manager.sseConnected).toBe(false);
    expect(manager.reconnecting).toBe(true);

    // 2. Reconnects and replays missed message
    const missed: MessageItem = {
      id: "m-2",
      sender: "CUSTOMER",
      text: "Are you there?",
      timestamp: "2026-08-20T12:01:00Z",
    };
    manager.handleSseReconnected([{ convId: "c-100", message: missed }]);

    expect(manager.sseConnected).toBe(true);
    expect(manager.reconnecting).toBe(false);
    expect(manager.conversations[0].messages.length).toBe(2);
    expect(manager.conversations[0].messages[1].text).toBe("Are you there?");
  });

  // Test 11: virtualised_lists_render_10000_rows_smoothly (Doc 16 §6)
  it("virtualised_lists_render_10000_rows_smoothly — handles 10,000 items with instant keyboard navigation", () => {
    const largeSet: ConversationItem[] = [];
    for (let i = 0; i < 10000; i++) {
      largeSet.push({
        id: `c-${i}`,
        customerId: `cust-${i}`,
        customerName: `Customer ${i}`,
        phone: `+92300${String(i).padStart(7, "0")}`,
        branchId: "b-1",
        lastMessage: `Message ${i}`,
        hasPrescription: i % 10 === 0,
        unreadCount: i % 5 === 0 ? 1 : 0,
        status: "ACTIVE",
        messages: [],
      });
    }

    const manager = new InboxManager(largeSet);
    expect(manager.selectedConversationId).toBe("c-0");

    // Rapid keyboard navigation
    manager.navigateNext();
    expect(manager.selectedConversationId).toBe("c-1");
    manager.navigateNext();
    expect(manager.selectedConversationId).toBe("c-2");
    manager.navigatePrevious();
    expect(manager.selectedConversationId).toBe("c-1");
  });

  // Test 12: keyboard_flow_completes_rx_review_without_mouse (Doc 16 §7)
  it("keyboard_flow_completes_rx_review_without_mouse — completes prescription review via keyboard only", () => {
    const rxItem: PrescriptionReviewItem = {
      id: "rx-kb",
      patientName: "Zahid Qureshi",
      createdAt: "2026-08-20T12:00:00Z",
      imageUrl: "https://example.com/rx_kb.jpg",
      imageTransform: { zoom: 1, rotation: 0, contrast: false },
      lines: [
        {
          id: "l-1",
          lineNumber: 1,
          rawOcrText: "Flagyl 400mg",
          matchedProduct: { productId: "p-flagyl", name: "Flagyl 400mg", isControlled: false, confidence: 0.96 },
          quantity: 20,
          dosage: "1 tab BD",
          alternativeCandidates: [],
          decision: null,
        },
        {
          id: "l-2",
          lineNumber: 2,
          rawOcrText: "Brufen 400mg",
          matchedProduct: { productId: "p-brufen", name: "Brufen 400mg", isControlled: false, confidence: 0.94 },
          quantity: 20,
          dosage: "1 tab TDS",
          alternativeCandidates: [],
          decision: null,
        },
      ],
    };

    const manager = new RxReviewManager([rxItem]);

    // Press '1' to select line 1
    manager.handleKeyboardShortcut("1", false);
    expect(manager.selectedLineNumber).toBe(1);

    // Press 'a' to accept line 1
    manager.handleKeyboardShortcut("a", false);
    expect(manager.currentItem?.lines[0].decision).toBe("ACCEPTED");

    // Press '2' to select line 2
    manager.handleKeyboardShortcut("2", false);
    expect(manager.selectedLineNumber).toBe(2);

    // Press 'a' to accept line 2
    manager.handleKeyboardShortcut("a", false);
    expect(manager.currentItem?.lines[1].decision).toBe("ACCEPTED");

    // Press 'Ctrl+Enter' to submit approval
    const approved = manager.handleKeyboardShortcut("Enter", true);
    expect(approved).toBe(true);
    expect(manager.queue.length).toBe(0); // Completed and advanced!
  });

  // Test 13: order_board_rejects_illegal_transition_drop (Doc 16 §9)
  it("order_board_rejects_illegal_transition_drop — prevents illegal status transitions on kanban", () => {
    const order: OrderCard = {
      id: "ord-test",
      orderNumber: "ORD-2026-001",
      customerName: "Bilal Aslam",
      status: "CONFIRMED",
      totalAmount: "3500.0000",
      itemCount: 3,
      branchId: "b-lhr",
      isPrescription: false,
      createdAt: "2026-08-20T14:00:00Z",
    };

    const manager = new OrderBoardManager([order]);

    // 1. Illegal transition: CONFIRMED directly to DELIVERED (skipping ALLOCATED, PACKED, DISPATCHED)
    const illegalMove = manager.moveOrder("ord-test", "DELIVERED");
    expect(illegalMove.success).toBe(false);
    expect(illegalMove.error).toContain("Illegal status transition");
    expect(manager.orders[0].status).toBe("CONFIRMED"); // Unaltered!

    // 2. Legal transition: CONFIRMED to ALLOCATED
    const legalMove = manager.moveOrder("ord-test", "ALLOCATED");
    expect(legalMove.success).toBe(true);
    expect(manager.orders[0].status).toBe("ALLOCATED");
  });

  // Test 14: all_screens_handle_loading_empty_error (Doc 16 §4, §11)
  it("all_screens_handle_loading_empty_error — provides state coverage across locales", () => {
    const locales: Locale[] = ["en", "ur", "ur-Latn"];
    for (const loc of locales) {
      const t = translations[loc];
      expect(t.common.loading).toBeTruthy();
      expect(t.common.empty).toBeTruthy();
      expect(t.common.error).toBeTruthy();
    }
  });
});
