/**
 * Order Fulfillment Kanban Board State (Doc 16 §9)
 * Validates legal order state transitions and prevents illegal drag-and-drop actions.
 */

export type OrderBoardStatus =
  | "PENDING_CONFIRMATION"
  | "RX_UNDER_REVIEW"
  | "PAYMENT_PENDING"
  | "CONFIRMED"
  | "ALLOCATED"
  | "PACKED"
  | "DISPATCHED"
  | "DELIVERED"
  | "CANCELLED";

export interface OrderCard {
  id: string;
  orderNumber: string;
  customerName: string;
  status: OrderBoardStatus;
  totalAmount: string;
  itemCount: number;
  branchId: string;
  isPrescription: boolean;
  createdAt: string;
}

// State transition table matching backend state machine
export const LEGAL_TRANSITIONS: Record<OrderBoardStatus, OrderBoardStatus[]> = {
  PENDING_CONFIRMATION: ["CONFIRMED", "RX_UNDER_REVIEW", "PAYMENT_PENDING", "CANCELLED"],
  RX_UNDER_REVIEW: ["CONFIRMED", "CANCELLED"],
  PAYMENT_PENDING: ["CONFIRMED", "CANCELLED"],
  CONFIRMED: ["ALLOCATED", "CANCELLED"],
  ALLOCATED: ["PACKED", "CANCELLED"],
  PACKED: ["DISPATCHED", "CANCELLED"],
  DISPATCHED: ["DELIVERED", "CANCELLED"],
  DELIVERED: [],
  CANCELLED: [],
};

export class OrderBoardManager {
  public orders: OrderCard[] = [];

  constructor(orders: OrderCard[] = []) {
    this.orders = orders;
  }

  public canTransition(currentStatus: OrderBoardStatus, targetStatus: OrderBoardStatus): boolean {
    const allowed = LEGAL_TRANSITIONS[currentStatus];
    return allowed ? allowed.includes(targetStatus) : false;
  }

  public moveOrder(orderId: string, targetStatus: OrderBoardStatus): { success: boolean; error?: string } {
    const order = this.orders.find((o) => o.id === orderId);
    if (!order) {
      return { success: false, error: "Order not found" };
    }

    if (!this.canTransition(order.status, targetStatus)) {
      return {
        success: false,
        error: `Illegal status transition from ${order.status} to ${targetStatus}`,
      };
    }

    order.status = targetStatus;
    return { success: true };
  }
}
