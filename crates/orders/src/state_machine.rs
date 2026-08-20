use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum OrderStatus {
    Draft,
    CartConfirmed,
    AwaitingRx,
    RxUnderReview,
    RxApproved,
    RxRejected,
    AwaitingPayment,
    PaymentUnderReview,
    PaymentRejected,
    Confirmed,
    Picking,
    Packed,
    Dispatched,
    OutForDelivery,
    Delivered,
    CashReconciled,
    Closed,
    Cancelled,
    FailedDelivery,
    Returned,
    Refunded,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::str::FromStr for OrderStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" | "DRAFT" => Ok(Self::Draft),
            "CartConfirmed" | "CART_CONFIRMED" => Ok(Self::CartConfirmed),
            "AwaitingRx" | "AWAITING_RX" => Ok(Self::AwaitingRx),
            "RxUnderReview" | "RX_UNDER_REVIEW" => Ok(Self::RxUnderReview),
            "RxApproved" | "RX_APPROVED" => Ok(Self::RxApproved),
            "RxRejected" | "RX_REJECTED" => Ok(Self::RxRejected),
            "AwaitingPayment" | "AWAITING_PAYMENT" => Ok(Self::AwaitingPayment),
            "PaymentUnderReview" | "PAYMENT_UNDER_REVIEW" => Ok(Self::PaymentUnderReview),
            "PaymentRejected" | "PAYMENT_REJECTED" => Ok(Self::PaymentRejected),
            "Confirmed" | "CONFIRMED" => Ok(Self::Confirmed),
            "Picking" | "PICKING" => Ok(Self::Picking),
            "Packed" | "PACKED" => Ok(Self::Packed),
            "Dispatched" | "DISPATCHED" => Ok(Self::Dispatched),
            "OutForDelivery" | "OUT_FOR_DELIVERY" => Ok(Self::OutForDelivery),
            "Delivered" | "DELIVERED" => Ok(Self::Delivered),
            "CashReconciled" | "CASH_RECONCILED" => Ok(Self::CashReconciled),
            "Closed" | "CLOSED" => Ok(Self::Closed),
            "Cancelled" | "CANCELLED" => Ok(Self::Cancelled),
            "FailedDelivery" | "FAILED_DELIVERY" => Ok(Self::FailedDelivery),
            "Returned" | "RETURNED" => Ok(Self::Returned),
            "Refunded" | "REFUNDED" => Ok(Self::Refunded),
            _ => Err(format!("Unknown order status: {}", s)),
        }
    }
}

/// Exhaustive state transition validation per Doc 10 §4.
pub fn can_transition(from: OrderStatus, to: OrderStatus) -> bool {
    use OrderStatus::*;
    matches!(
        (from, to),
        (Draft, CartConfirmed)
            | (Draft, Cancelled)
            | (CartConfirmed, AwaitingRx)
            | (CartConfirmed, AwaitingPayment)
            | (CartConfirmed, Cancelled)
            | (AwaitingRx, RxUnderReview)
            | (AwaitingRx, Cancelled)
            | (RxUnderReview, RxApproved)
            | (RxUnderReview, RxRejected)
            | (RxApproved, AwaitingPayment)
            | (RxRejected, Cancelled)
            | (AwaitingPayment, PaymentUnderReview)
            | (AwaitingPayment, Confirmed)
            | (AwaitingPayment, Cancelled)
            | (PaymentUnderReview, Confirmed)
            | (PaymentUnderReview, PaymentRejected)
            | (PaymentRejected, AwaitingPayment)
            | (PaymentRejected, Cancelled)
            | (Confirmed, Picking)
            | (Confirmed, Cancelled)
            | (Picking, Packed)
            | (Picking, Cancelled)
            | (Packed, Dispatched)
            | (Packed, Cancelled)
            | (Dispatched, OutForDelivery)
            | (OutForDelivery, Delivered)
            | (OutForDelivery, FailedDelivery)
            | (Delivered, CashReconciled)
            | (Delivered, Closed)
            | (Delivered, Returned)
            | (CashReconciled, Closed)
            | (FailedDelivery, OutForDelivery)
            | (FailedDelivery, Returned)
            | (Returned, Refunded)
            | (Returned, Closed)
            | (Refunded, Closed)
    )
}
