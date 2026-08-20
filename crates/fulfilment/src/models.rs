use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shifa_core::id::{
    BranchId, DeliveryId, OrderId, PickingListId, RiderCashSessionId, RiderId, TenantId, UserId,
};
use shifa_core::money::Money;
use std::fmt;
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum DeliveryStatus {
    Unassigned,
    Assigned,
    Accepted,
    PickedUp,
    InTransit,
    Delivered,
    Failed,
    Returned,
}

impl fmt::Display for DeliveryStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unassigned => write!(f, "UNASSIGNED"),
            Self::Assigned => write!(f, "ASSIGNED"),
            Self::Accepted => write!(f, "ACCEPTED"),
            Self::PickedUp => write!(f, "PICKED_UP"),
            Self::InTransit => write!(f, "IN_TRANSIT"),
            Self::Delivered => write!(f, "DELIVERED"),
            Self::Failed => write!(f, "FAILED"),
            Self::Returned => write!(f, "RETURNED"),
        }
    }
}

impl FromStr for DeliveryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "UNASSIGNED" | "PENDING" => Ok(Self::Unassigned),
            "ASSIGNED" => Ok(Self::Assigned),
            "ACCEPTED" => Ok(Self::Accepted),
            "PICKED_UP" => Ok(Self::PickedUp),
            "IN_TRANSIT" | "OUT_FOR_DELIVERY" => Ok(Self::InTransit),
            "DELIVERED" => Ok(Self::Delivered),
            "FAILED" => Ok(Self::Failed),
            "RETURNED" => Ok(Self::Returned),
            other => Err(format!("Unknown delivery status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum RiderStatus {
    Available,
    Busy,
    OffDuty,
    Suspended,
}

impl fmt::Display for RiderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Available => write!(f, "AVAILABLE"),
            Self::Busy => write!(f, "BUSY"),
            Self::OffDuty => write!(f, "OFF_DUTY"),
            Self::Suspended => write!(f, "SUSPENDED"),
        }
    }
}

impl FromStr for RiderStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AVAILABLE" => Ok(Self::Available),
            "BUSY" => Ok(Self::Busy),
            "OFF_DUTY" => Ok(Self::OffDuty),
            "SUSPENDED" => Ok(Self::Suspended),
            other => Err(format!("Unknown rider status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum CashSessionStatus {
    Open,
    Declared,
    Reconciled,
}

impl fmt::Display for CashSessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "OPEN"),
            Self::Declared => write!(f, "DECLARED"),
            Self::Reconciled => write!(f, "RECONCILED"),
        }
    }
}

impl FromStr for CashSessionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "OPEN" => Ok(Self::Open),
            "DECLARED" => Ok(Self::Declared),
            "RECONCILED" => Ok(Self::Reconciled),
            other => Err(format!("Unknown cash session status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub enum PickingListStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl fmt::Display for PickingListStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::InProgress => write!(f, "IN_PROGRESS"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Cancelled => write!(f, "CANCELLED"),
        }
    }
}

impl FromStr for PickingListStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PENDING" => Ok(Self::Pending),
            "IN_PROGRESS" => Ok(Self::InProgress),
            "COMPLETED" => Ok(Self::Completed),
            "CANCELLED" => Ok(Self::Cancelled),
            other => Err(format!("Unknown picking list status: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RiderDto {
    pub id: RiderId,
    pub tenant_id: TenantId,
    pub branch_id: BranchId,
    pub user_id: UserId,
    pub vehicle_type: String,
    pub cnic: String,
    pub licence_no: String,
    pub status: RiderStatus,
    pub on_shift: bool,
    pub decline_count: i32,
    pub shift_started_at: Option<DateTime<Utc>>,
    pub shift_ended_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeliveryDto {
    pub id: DeliveryId,
    pub tenant_id: TenantId,
    pub branch_id: Option<BranchId>,
    pub order_id: OrderId,
    pub rider_id: Option<RiderId>,
    pub status: DeliveryStatus,
    pub assigned_at: Option<DateTime<Utc>>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub picked_up_at: Option<DateTime<Utc>>,
    pub in_transit_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
    pub failed_reason: Option<String>,
    pub decline_reason: Option<String>,
    pub pod_image_object_key: Option<String>,
    pub pod_signature_object_key: Option<String>,
    pub recipient_name: Option<String>,
    pub recipient_cnic_last4: Option<String>,
    pub prescription_collected: bool,
    pub cash_collected: Option<Money>,
    pub reattempt_count: i32,
    pub tracking_token: String,
    pub gps_denied_flag: bool,
    pub distance_km: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RiderCashSessionDto {
    pub id: RiderCashSessionId,
    pub tenant_id: TenantId,
    pub rider_id: RiderId,
    pub branch_id: Option<BranchId>,
    pub status: CashSessionStatus,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub expected_amount: Money,
    pub collected_amount: Money,
    pub deposited_amount: Money,
    pub variance: Money,
    pub reconciled_by: Option<UserId>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PickingListDto {
    pub id: PickingListId,
    pub tenant_id: TenantId,
    pub branch_id: BranchId,
    pub order_id: OrderId,
    pub status: PickingListStatus,
    pub items: serde_json::Value,
    pub picked_by: Option<UserId>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Zero PII Public Customer Tracking Payload (Doc 12 §8, §10)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PublicTrackingDto {
    pub order_ref: String,
    pub status: DeliveryStatus,
    pub branch_name: String,
    pub estimated_delivery_time: Option<DateTime<Utc>>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub picked_up_at: Option<DateTime<Utc>>,
    pub delivered_at: Option<DateTime<Utc>>,
}

// Request Types
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateRiderRequest {
    pub branch_id: BranchId,
    pub user_id: UserId,
    pub vehicle_type: Option<String>,
    pub cnic: String,
    pub licence_no: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssignDeliveryRequest {
    pub rider_id: RiderId,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeclineDeliveryRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeliverRequest {
    pub pod_image_object_key: String,
    pub pod_signature_object_key: Option<String>,
    pub recipient_name: String,
    pub recipient_cnic_last4: Option<String>,
    pub prescription_collected: Option<bool>,
    pub cash_collected: Option<Money>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub gps_denied: Option<bool>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FailDeliveryRequest {
    pub reason: String,
    pub photo_object_key: Option<String>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeclareCashRequest {
    pub collected_amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReconcileCashSessionRequest {
    pub deposited_amount: Money,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VarianceReportItem {
    pub rider_id: RiderId,
    pub rider_name: String,
    pub branch_id: BranchId,
    pub total_expected: Money,
    pub total_collected: Money,
    pub total_deposited: Money,
    pub total_variance: Money,
    pub session_count: i64,
    pub unresolved_sessions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VarianceReportDto {
    pub start_date: String,
    pub end_date: String,
    pub branch_id: Option<BranchId>,
    pub items: Vec<VarianceReportItem>,
}
