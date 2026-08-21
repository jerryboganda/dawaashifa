use chrono::{DateTime, Duration, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use shifa_core::context::TenantContext;
use shifa_core::id::{
    BranchId, DeliveryId, OrderId, PickingListId, RiderCashSessionId, RiderId, TenantId, UserId,
};
use shifa_core::money::Money;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::assignment::AssignmentEngine;
use crate::error::FulfilmentError;
use crate::models::*;

#[derive(Clone)]
pub struct FulfilmentService {
    pool: PgPool,
}

impl FulfilmentService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // --------------------------------------------------------------------------------------------
    // Rider Management
    // --------------------------------------------------------------------------------------------

    pub async fn create_rider(
        &self,
        ctx: &TenantContext,
        req: CreateRiderRequest,
    ) -> Result<RiderDto, FulfilmentError> {
        ctx.require("user.create")
            .map_err(|e| FulfilmentError::Unauthorized(e.to_string()))?;

        let rider_id = RiderId::new();
        let vehicle = req.vehicle_type.unwrap_or_else(|| "MOTORBIKE".into());

        sqlx::query(
            "INSERT INTO riders (id, tenant_id, branch_id, user_id, vehicle_type, cnic, licence_no, status, on_shift)
             VALUES ($1, $2, $3, $4, $5, $6, $7, 'AVAILABLE'::rider_status, false)"
        )
        .bind(rider_id.0)
        .bind(ctx.tenant_id().0)
        .bind(req.branch_id.0)
        .bind(req.user_id.0)
        .bind(&vehicle)
        .bind(&req.cnic)
        .bind(&req.licence_no)
        .execute(&self.pool)
        .await?;

        self.get_rider(ctx, rider_id).await
    }

    pub async fn get_rider(
        &self,
        ctx: &TenantContext,
        rider_id: RiderId,
    ) -> Result<RiderDto, FulfilmentError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, branch_id, user_id, vehicle_type, cnic, licence_no,
                    status::text as status, on_shift, decline_count, shift_started_at, shift_ended_at,
                    created_at, updated_at
             FROM riders
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(rider_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FulfilmentError::RiderNotFound(rider_id))?;

        self.map_rider_row(row)
    }

    pub async fn get_rider_by_user_id(
        &self,
        ctx: &TenantContext,
        user_id: UserId,
    ) -> Result<Option<RiderDto>, FulfilmentError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, branch_id, user_id, vehicle_type, cnic, licence_no,
                    status::text as status, on_shift, decline_count, shift_started_at, shift_ended_at,
                    created_at, updated_at
             FROM riders
             WHERE tenant_id = $1 AND user_id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(user_id.0)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => self.map_rider_row(r).map(Some),
            None => Ok(None),
        }
    }

    pub async fn list_riders(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        status: Option<RiderStatus>,
        on_shift: Option<bool>,
    ) -> Result<Vec<RiderDto>, FulfilmentError> {
        // Enforce rider token scoping: riders cannot list other riders
        if ctx.role_names().iter().any(|r| r == "RIDER")
            && !ctx
                .role_names()
                .iter()
                .any(|r| r == "SUPER_ADMIN" || r == "BRANCH_MANAGER")
        {
            return Err(FulfilmentError::Forbidden(
                "Riders are not permitted to list other riders".into(),
            ));
        }

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, tenant_id, branch_id, user_id, vehicle_type, cnic, licence_no,
                    status::text as status, on_shift, decline_count, shift_started_at, shift_ended_at,
                    created_at, updated_at
             FROM riders
             WHERE tenant_id = "
        );
        query_builder.push_bind(ctx.tenant_id().0);

        if let Some(bid) = branch_id {
            query_builder.push(" AND branch_id = ");
            query_builder.push_bind(bid.0);
        }

        if let Some(st) = status {
            query_builder.push(" AND status = ");
            query_builder.push_bind(st.to_string());
            query_builder.push("::rider_status");
        }

        if let Some(os) = on_shift {
            query_builder.push(" AND on_shift = ");
            query_builder.push_bind(os);
        }

        query_builder.push(" ORDER BY created_at DESC");

        let rows = query_builder.build().fetch_all(&self.pool).await?;
        let mut list = Vec::new();
        for row in rows {
            list.push(self.map_rider_row(row)?);
        }
        Ok(list)
    }

    pub async fn start_shift(
        &self,
        ctx: &TenantContext,
        rider_id: RiderId,
    ) -> Result<RiderDto, FulfilmentError> {
        self.verify_rider_or_admin(ctx, rider_id).await?;

        sqlx::query(
            "UPDATE riders SET
                on_shift = true,
                status = 'AVAILABLE'::rider_status,
                shift_started_at = now(),
                updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(rider_id.0)
        .execute(&self.pool)
        .await?;

        self.get_rider(ctx, rider_id).await
    }

    pub async fn end_shift(
        &self,
        ctx: &TenantContext,
        rider_id: RiderId,
    ) -> Result<RiderDto, FulfilmentError> {
        self.verify_rider_or_admin(ctx, rider_id).await?;

        sqlx::query(
            "UPDATE riders SET
                on_shift = false,
                status = 'OFF_DUTY'::rider_status,
                shift_ended_at = now(),
                updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(rider_id.0)
        .execute(&self.pool)
        .await?;

        self.get_rider(ctx, rider_id).await
    }

    // --------------------------------------------------------------------------------------------
    // Picking Lists
    // --------------------------------------------------------------------------------------------

    pub async fn create_picking_list(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
        order_id: OrderId,
        items: serde_json::Value,
    ) -> Result<PickingListDto, FulfilmentError> {
        let id = PickingListId::new();

        sqlx::query(
            "INSERT INTO picking_lists (id, tenant_id, branch_id, order_id, status, items)
             VALUES ($1, $2, $3, $4, 'PENDING', $5)",
        )
        .bind(id.0)
        .bind(ctx.tenant_id().0)
        .bind(branch_id.0)
        .bind(order_id.0)
        .bind(items)
        .execute(&self.pool)
        .await?;

        self.get_picking_list(ctx, id).await
    }

    pub async fn get_picking_list(
        &self,
        ctx: &TenantContext,
        id: PickingListId,
    ) -> Result<PickingListDto, FulfilmentError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, branch_id, order_id, status, items, picked_by, completed_at, created_at, updated_at
             FROM picking_lists
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FulfilmentError::PickingListNotFound(id))?;

        self.map_picking_list_row(row)
    }

    pub async fn list_picking_lists(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        status: Option<PickingListStatus>,
    ) -> Result<Vec<PickingListDto>, FulfilmentError> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, tenant_id, branch_id, order_id, status, items, picked_by, completed_at, created_at, updated_at
             FROM picking_lists
             WHERE tenant_id = "
        );
        query_builder.push_bind(ctx.tenant_id().0);

        if let Some(bid) = branch_id {
            query_builder.push(" AND branch_id = ");
            query_builder.push_bind(bid.0);
        }

        if let Some(st) = status {
            query_builder.push(" AND status = ");
            query_builder.push_bind(st.to_string());
        }

        query_builder.push(" ORDER BY created_at DESC");

        let rows = query_builder.build().fetch_all(&self.pool).await?;
        let mut list = Vec::new();
        for row in rows {
            list.push(self.map_picking_list_row(row)?);
        }
        Ok(list)
    }

    pub async fn complete_picking_list(
        &self,
        ctx: &TenantContext,
        id: PickingListId,
    ) -> Result<PickingListDto, FulfilmentError> {
        let user_id = ctx.user_id();

        sqlx::query(
            "UPDATE picking_lists SET
                status = 'COMPLETED',
                picked_by = $1,
                completed_at = now(),
                updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(user_id.0)
        .bind(ctx.tenant_id().0)
        .bind(id.0)
        .execute(&self.pool)
        .await?;

        self.get_picking_list(ctx, id).await
    }

    // --------------------------------------------------------------------------------------------
    // Delivery Operations & Lifecycle
    // --------------------------------------------------------------------------------------------

    pub async fn create_delivery_for_order(
        &self,
        ctx: &TenantContext,
        branch_id: BranchId,
        order_id: OrderId,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery_id = DeliveryId::new();
        let token = format!("trk_{}", Uuid::now_v7().simple());

        sqlx::query(
            "INSERT INTO deliveries (id, tenant_id, branch_id, order_id, status, tracking_token)
             VALUES ($1, $2, $3, $4, 'UNASSIGNED'::delivery_status, $5)",
        )
        .bind(delivery_id.0)
        .bind(ctx.tenant_id().0)
        .bind(branch_id.0)
        .bind(order_id.0)
        .bind(token)
        .execute(&self.pool)
        .await?;

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn get_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, branch_id, order_id, rider_id, status::text as status,
                    assigned_at, accepted_at, picked_up_at, in_transit_at, delivered_at,
                    failed_reason, decline_reason, pod_image_object_key, pod_signature_object_key,
                    recipient_name, recipient_cnic_last4, prescription_collected, cash_collected,
                    reattempt_count, tracking_token, gps_denied_flag, distance_km, created_at, updated_at
             FROM deliveries
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FulfilmentError::DeliveryNotFound(delivery_id))?;

        let dto = self.map_delivery_row(row)?;

        // Enforce rider token scoping (Doc 12 §8, §10):
        // If caller is a rider, they can ONLY read their own assigned deliveries.
        if ctx.role_names().iter().any(|r| r == "RIDER")
            && !ctx
                .role_names()
                .iter()
                .any(|r| r == "SUPER_ADMIN" || r == "BRANCH_MANAGER")
        {
            let rider = self.get_rider_by_user_id(ctx, ctx.user_id()).await?;
            if let Some(r) = rider {
                if dto.rider_id != Some(r.id) {
                    return Err(FulfilmentError::Forbidden(
                        "Rider token cannot read other riders' deliveries".into(),
                    ));
                }
            } else {
                return Err(FulfilmentError::Forbidden(
                    "Rider record not found for user".into(),
                ));
            }
        }

        Ok(dto)
    }

    pub async fn list_deliveries(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        rider_id: Option<RiderId>,
        status: Option<DeliveryStatus>,
        date: Option<NaiveDate>,
    ) -> Result<Vec<DeliveryDto>, FulfilmentError> {
        // Enforce rider token scoping (Doc 12 §8, §10):
        let effective_rider_id = if ctx.role_names().iter().any(|r| r == "RIDER")
            && !ctx
                .role_names()
                .iter()
                .any(|r| r == "SUPER_ADMIN" || r == "BRANCH_MANAGER")
        {
            let rider = self.get_rider_by_user_id(ctx, ctx.user_id()).await?;
            if let Some(r) = rider {
                Some(r.id)
            } else {
                return Err(FulfilmentError::Forbidden(
                    "Rider record not found for user".into(),
                ));
            }
        } else {
            rider_id
        };

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, tenant_id, branch_id, order_id, rider_id, status::text as status,
                    assigned_at, accepted_at, picked_up_at, in_transit_at, delivered_at,
                    failed_reason, decline_reason, pod_image_object_key, pod_signature_object_key,
                    recipient_name, recipient_cnic_last4, prescription_collected, cash_collected,
                    reattempt_count, tracking_token, gps_denied_flag, distance_km, created_at, updated_at
             FROM deliveries
             WHERE tenant_id = "
        );
        query_builder.push_bind(ctx.tenant_id().0);

        if let Some(bid) = branch_id {
            query_builder.push(" AND branch_id = ");
            query_builder.push_bind(bid.0);
        }

        if let Some(rid) = effective_rider_id {
            query_builder.push(" AND rider_id = ");
            query_builder.push_bind(rid.0);
        }

        if let Some(st) = status {
            query_builder.push(" AND status = ");
            query_builder.push_bind(st.to_string());
            query_builder.push("::delivery_status");
        }

        if let Some(d) = date {
            query_builder.push(" AND created_at::date = ");
            query_builder.push_bind(d);
        }

        query_builder.push(" ORDER BY created_at DESC");

        let rows = query_builder.build().fetch_all(&self.pool).await?;
        let mut list = Vec::new();
        for row in rows {
            list.push(self.map_delivery_row(row)?);
        }
        Ok(list)
    }

    pub async fn assign_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
        rider_id: RiderId,
    ) -> Result<DeliveryDto, FulfilmentError> {
        ctx.require("order.edit")
            .map_err(|e| FulfilmentError::Unauthorized(e.to_string()))?;

        let delivery = self.get_delivery(ctx, delivery_id).await?;
        let rider = self.get_rider(ctx, rider_id).await?;

        // 1. Fetch order details to check if COD
        let order_row = sqlx::query(
            "SELECT total_amount, payment_method FROM orders WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery.order_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FulfilmentError::NotFound(
            "Associated order not found".into(),
        ))?;

        let is_cod = order_row
            .get::<Option<String>, _>("payment_method")
            .as_deref()
            == Some("COD");
        let total_amount_dec: Decimal = order_row.get("total_amount");
        let order_amount = Money::from_decimal(total_amount_dec);

        // 2. Financial Safety Constraint (Doc 12 §5, §10):
        // If COD order, check rider's undeposited cash against COD ceiling and ensure no stale session (>24h)
        if is_cod {
            let default_cod_ceiling = Money::from_major(10000); // Rs 10,000 default branch ceiling
            AssignmentEngine::validate_cod_assignment_eligibility(
                &self.pool,
                ctx,
                rider_id,
                rider.branch_id,
                order_amount,
                default_cod_ceiling,
            )
            .await?;
        }

        // 3. Update delivery to ASSIGNED
        sqlx::query(
            "UPDATE deliveries SET
                rider_id = $1,
                status = 'ASSIGNED'::delivery_status,
                assigned_at = now(),
                updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(rider_id.0)
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn accept_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery = self.get_delivery(ctx, delivery_id).await?;
        if let Some(rid) = delivery.rider_id {
            self.verify_rider_or_admin(ctx, rid).await?;
        }

        sqlx::query(
            "UPDATE deliveries SET
                status = 'ACCEPTED'::delivery_status,
                accepted_at = now(),
                updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn decline_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
        req: DeclineDeliveryRequest,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery = self.get_delivery(ctx, delivery_id).await?;
        if let Some(rid) = delivery.rider_id {
            self.verify_rider_or_admin(ctx, rid).await?;

            // Increment rider's decline_count
            sqlx::query(
                "UPDATE riders SET decline_count = decline_count + 1, updated_at = now() WHERE tenant_id = $1 AND id = $2"
            )
            .bind(ctx.tenant_id().0)
            .bind(rid.0)
            .execute(&self.pool)
            .await?;
        }

        // Return delivery to UNASSIGNED with recorded decline_reason
        sqlx::query(
            "UPDATE deliveries SET
                rider_id = NULL,
                status = 'UNASSIGNED'::delivery_status,
                decline_reason = $1,
                updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(&req.reason)
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            delivery_id.0,
            "DELIVERY_DECLINED",
            json!({ "reason": req.reason, "order_id": delivery.order_id.0 }),
        )
        .await?;

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn pickup_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery = self.get_delivery(ctx, delivery_id).await?;
        if let Some(rid) = delivery.rider_id {
            self.verify_rider_or_admin(ctx, rid).await?;
        }

        sqlx::query(
            "UPDATE deliveries SET
                status = 'PICKED_UP'::delivery_status,
                picked_up_at = now(),
                updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn start_in_transit(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery = self.get_delivery(ctx, delivery_id).await?;
        if let Some(rid) = delivery.rider_id {
            self.verify_rider_or_admin(ctx, rid).await?;
        }

        sqlx::query(
            "UPDATE deliveries SET
                status = 'IN_TRANSIT'::delivery_status,
                in_transit_at = now(),
                updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn complete_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
        req: DeliverRequest,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery = self.get_delivery(ctx, delivery_id).await?;
        if let Some(rid) = delivery.rider_id {
            self.verify_rider_or_admin(ctx, rid).await?;
        }

        // Idempotency: If already DELIVERED, return current delivery safely
        if delivery.status == DeliveryStatus::Delivered {
            return Ok(delivery);
        }

        // 1. Mandatory POD Validations (Doc 12 §6, §10):
        if req.pod_image_object_key.trim().is_empty() {
            return Err(FulfilmentError::PodMissingField(
                "Photo is mandatory for delivery completion".into(),
            ));
        }

        if req.recipient_name.trim().is_empty() {
            return Err(FulfilmentError::PodMissingField(
                "Recipient name is mandatory for delivery completion".into(),
            ));
        }

        let gps_denied = req.gps_denied.unwrap_or(false);
        if !gps_denied && (req.latitude.is_none() || req.longitude.is_none()) {
            return Err(FulfilmentError::PodMissingField(
                "GPS coordinates or explicit gps_denied flag is required".into(),
            ));
        }

        // 2. Controlled Substance Order Extra Validation (Doc 12 §6, §10):
        // Check if any product in order is marked prescription_only or controlled
        let has_controlled: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM order_items oi
                JOIN products p ON p.id = oi.product_id AND p.tenant_id = oi.tenant_id
                WHERE oi.tenant_id = $1 AND oi.order_id = $2 AND (p.is_prescription_only = true OR p.is_controlled = true)
            )"
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery.order_id.0)
        .fetch_one(&self.pool)
        .await?;

        if has_controlled {
            let rx_collected = req.prescription_collected.unwrap_or(false);
            let cnic_last4 = req.recipient_cnic_last4.as_deref().unwrap_or("");
            if !rx_collected
                || cnic_last4.len() != 4
                || !cnic_last4.chars().all(|c| c.is_ascii_digit())
            {
                return Err(FulfilmentError::ControlledSubstanceRequiresPrescriptionAndCnic);
            }
        }

        // 3. COD Cash Session accumulation (Doc 12 §7, §10):
        let cash_collected_money = req.cash_collected.unwrap_or_else(Money::zero);
        if let Some(rider_id) = delivery.rider_id {
            if cash_collected_money.0 > Decimal::ZERO {
                self.accumulate_cod_cash(ctx, rider_id, delivery.branch_id, cash_collected_money)
                    .await?;
            }
        }

        // 4. Update delivery to DELIVERED
        sqlx::query(
            "UPDATE deliveries SET
                status = 'DELIVERED'::delivery_status,
                delivered_at = now(),
                pod_image_object_key = $1,
                pod_signature_object_key = $2,
                recipient_name = $3,
                recipient_cnic_last4 = $4,
                prescription_collected = $5,
                cash_collected = $6,
                gps_denied_flag = $7,
                idempotency_key = $8,
                updated_at = now()
             WHERE tenant_id = $9 AND id = $10",
        )
        .bind(&req.pod_image_object_key)
        .bind(&req.pod_signature_object_key)
        .bind(&req.recipient_name)
        .bind(&req.recipient_cnic_last4)
        .bind(req.prescription_collected.unwrap_or(false))
        .bind(cash_collected_money.0)
        .bind(gps_denied)
        .bind(&req.idempotency_key)
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        // 5. Advance order status to DELIVERED
        sqlx::query(
            "UPDATE orders SET status = 'DELIVERED'::order_status, updated_at = now()
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(delivery.order_id.0)
        .execute(&self.pool)
        .await
        .ok();

        self.get_delivery(ctx, delivery_id).await
    }

    pub async fn fail_delivery(
        &self,
        ctx: &TenantContext,
        delivery_id: DeliveryId,
        req: FailDeliveryRequest,
    ) -> Result<DeliveryDto, FulfilmentError> {
        let delivery = self.get_delivery(ctx, delivery_id).await?;
        if let Some(rid) = delivery.rider_id {
            self.verify_rider_or_admin(ctx, rid).await?;
        }

        // Idempotency: If already FAILED or RETURNED with this idempotency key, return safely
        if let Some(ref key) = req.idempotency_key {
            let existing_idempotent: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM deliveries WHERE tenant_id = $1 AND id = $2 AND idempotency_key = $3"
            )
            .bind(ctx.tenant_id().0)
            .bind(delivery_id.0)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;

            if existing_idempotent.is_some() {
                return self.get_delivery(ctx, delivery_id).await;
            }
        }

        let new_reattempt_count = delivery.reattempt_count + 1;
        // Doc 12 §4, §10: Max 2 reattempts (3 total failures), then marked RETURNED
        let new_status = if new_reattempt_count > 2 {
            DeliveryStatus::Returned
        } else {
            DeliveryStatus::Failed
        };

        sqlx::query(
            "UPDATE deliveries SET
                status = $1::delivery_status,
                failed_reason = $2,
                reattempt_count = $3,
                idempotency_key = $4,
                updated_at = now()
             WHERE tenant_id = $5 AND id = $6",
        )
        .bind(new_status.to_string())
        .bind(&req.reason)
        .bind(new_reattempt_count)
        .bind(&req.idempotency_key)
        .bind(ctx.tenant_id().0)
        .bind(delivery_id.0)
        .execute(&self.pool)
        .await?;

        // If returned, advance order status
        if new_status == DeliveryStatus::Returned {
            sqlx::query(
                "UPDATE orders SET status = 'RETURNED'::order_status, updated_at = now()
                 WHERE tenant_id = $1 AND id = $2",
            )
            .bind(ctx.tenant_id().0)
            .bind(delivery.order_id.0)
            .execute(&self.pool)
            .await
            .ok();
        }

        self.get_delivery(ctx, delivery_id).await
    }

    // --------------------------------------------------------------------------------------------
    // Cash Sessions & Daily Reconciliation
    // --------------------------------------------------------------------------------------------

    async fn accumulate_cod_cash(
        &self,
        ctx: &TenantContext,
        rider_id: RiderId,
        branch_id: Option<BranchId>,
        amount: Money,
    ) -> Result<(), FulfilmentError> {
        // Find existing open session or open new one
        let existing_session = sqlx::query(
            "SELECT id, expected_amount FROM rider_cash_sessions
             WHERE tenant_id = $1 AND rider_id = $2 AND status = 'OPEN'
             ORDER BY opened_at DESC LIMIT 1",
        )
        .bind(ctx.tenant_id().0)
        .bind(rider_id.0)
        .fetch_optional(&self.pool)
        .await?;

        match existing_session {
            Some(row) => {
                let session_id: Uuid = row.get("id");
                let cur_exp_dec: Decimal = row.get("expected_amount");
                let new_exp = Money::from_decimal(cur_exp_dec + amount.0);

                sqlx::query(
                    "UPDATE rider_cash_sessions SET expected_amount = $1 WHERE tenant_id = $2 AND id = $3"
                )
                .bind(new_exp.0)
                .bind(ctx.tenant_id().0)
                .bind(session_id)
                .execute(&self.pool)
                .await?;
            }
            None => {
                let session_id = RiderCashSessionId::new();
                let bid = branch_id.map(|b| b.0).unwrap_or_default();

                sqlx::query(
                    "INSERT INTO rider_cash_sessions (id, tenant_id, rider_id, branch_id, status, opened_at, expected_amount)
                     VALUES ($1, $2, $3, $4, 'OPEN', now(), $5)"
                )
                .bind(session_id.0)
                .bind(ctx.tenant_id().0)
                .bind(rider_id.0)
                .bind(bid)
                .bind(amount.0)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_cash_session(
        &self,
        ctx: &TenantContext,
        session_id: RiderCashSessionId,
    ) -> Result<RiderCashSessionDto, FulfilmentError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, rider_id, branch_id, status, opened_at, closed_at,
                    expected_amount, collected_amount, deposited_amount, variance, reconciled_by, note
             FROM rider_cash_sessions
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id().0)
        .bind(session_id.0)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FulfilmentError::CashSessionNotFound(session_id))?;

        self.map_cash_session_row(row)
    }

    pub async fn list_cash_sessions(
        &self,
        ctx: &TenantContext,
        rider_id: Option<RiderId>,
        branch_id: Option<BranchId>,
        status: Option<CashSessionStatus>,
    ) -> Result<Vec<RiderCashSessionDto>, FulfilmentError> {
        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT id, tenant_id, rider_id, branch_id, status, opened_at, closed_at,
                    expected_amount, collected_amount, deposited_amount, variance, reconciled_by, note
             FROM rider_cash_sessions
             WHERE tenant_id = "
        );
        query_builder.push_bind(ctx.tenant_id().0);

        if let Some(rid) = rider_id {
            query_builder.push(" AND rider_id = ");
            query_builder.push_bind(rid.0);
        }

        if let Some(bid) = branch_id {
            query_builder.push(" AND branch_id = ");
            query_builder.push_bind(bid.0);
        }

        if let Some(st) = status {
            query_builder.push(" AND status = ");
            query_builder.push_bind(st.to_string());
        }

        query_builder.push(" ORDER BY opened_at DESC");

        let rows = query_builder.build().fetch_all(&self.pool).await?;
        let mut list = Vec::new();
        for row in rows {
            list.push(self.map_cash_session_row(row)?);
        }
        Ok(list)
    }

    pub async fn declare_cash(
        &self,
        ctx: &TenantContext,
        session_id: RiderCashSessionId,
        req: DeclareCashRequest,
    ) -> Result<RiderCashSessionDto, FulfilmentError> {
        let session = self.get_cash_session(ctx, session_id).await?;
        self.verify_rider_or_admin(ctx, session.rider_id).await?;

        sqlx::query(
            "UPDATE rider_cash_sessions SET
                status = 'DECLARED',
                collected_amount = $1
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(req.collected_amount.0)
        .bind(ctx.tenant_id().0)
        .bind(session_id.0)
        .execute(&self.pool)
        .await?;

        self.get_cash_session(ctx, session_id).await
    }

    pub async fn reconcile_cash_session(
        &self,
        ctx: &TenantContext,
        session_id: RiderCashSessionId,
        req: ReconcileCashSessionRequest,
    ) -> Result<RiderCashSessionDto, FulfilmentError> {
        ctx.require("payment.approve")
            .map_err(|e| FulfilmentError::Unauthorized(e.to_string()))?;

        let session = self.get_cash_session(ctx, session_id).await?;
        let variance = Money::from_decimal(req.deposited_amount.0 - session.expected_amount.0);

        // Doc 12 §7, §10: Non-zero variance requires a documented reason note and blocks closure
        if variance.0 != Decimal::ZERO {
            let note_str = req.note.as_deref().unwrap_or("").trim();
            if note_str.is_empty() {
                return Err(FulfilmentError::VarianceReasonRequired);
            }
        }

        let user_id = ctx.user_id();

        sqlx::query(
            "UPDATE rider_cash_sessions SET
                status = 'RECONCILED',
                deposited_amount = $1,
                variance = $2,
                note = $3,
                reconciled_by = $4,
                closed_at = now()
             WHERE tenant_id = $5 AND id = $6",
        )
        .bind(req.deposited_amount.0)
        .bind(variance.0)
        .bind(&req.note)
        .bind(user_id.0)
        .bind(ctx.tenant_id().0)
        .bind(session_id.0)
        .execute(&self.pool)
        .await?;

        self.write_audit_log(
            ctx,
            session_id.0,
            "CASH_SESSION_RECONCILED",
            json!({
                "rider_id": session.rider_id.0,
                "expected": session.expected_amount.0,
                "deposited": req.deposited_amount.0,
                "variance": variance.0,
                "note": req.note
            }),
        )
        .await?;

        self.get_cash_session(ctx, session_id).await
    }

    pub async fn get_variance_report(
        &self,
        ctx: &TenantContext,
        start_date: NaiveDate,
        end_date: NaiveDate,
        branch_id: Option<BranchId>,
    ) -> Result<VarianceReportDto, FulfilmentError> {
        ctx.require("report.view")
            .map_err(|e| FulfilmentError::Unauthorized(e.to_string()))?;

        let mut query_builder = sqlx::QueryBuilder::new(
            "SELECT s.rider_id, COALESCE(u.full_name, 'Unknown') as rider_name, s.branch_id,
                    COALESCE(SUM(s.expected_amount), 0.0000) as total_expected,
                    COALESCE(SUM(s.collected_amount), 0.0000) as total_collected,
                    COALESCE(SUM(s.deposited_amount), 0.0000) as total_deposited,
                    COALESCE(SUM(s.variance), 0.0000) as total_variance,
                    COUNT(s.id) as session_count,
                    COUNT(s.id) FILTER (WHERE s.status != 'RECONCILED') as unresolved_sessions
             FROM rider_cash_sessions s
             JOIN riders r ON r.id = s.rider_id AND r.tenant_id = s.tenant_id
             LEFT JOIN users u ON u.id = r.user_id AND u.tenant_id = r.tenant_id
             WHERE s.tenant_id = ",
        );
        query_builder.push_bind(ctx.tenant_id().0);
        query_builder.push(" AND s.opened_at::date >= ");
        query_builder.push_bind(start_date);
        query_builder.push(" AND s.opened_at::date <= ");
        query_builder.push_bind(end_date);

        if let Some(bid) = branch_id {
            query_builder.push(" AND s.branch_id = ");
            query_builder.push_bind(bid.0);
        }

        query_builder
            .push(" GROUP BY s.rider_id, u.full_name, s.branch_id ORDER BY total_variance ASC");

        let rows = query_builder.build().fetch_all(&self.pool).await?;
        let mut items = Vec::new();
        for row in rows {
            let rider_id: Uuid = row.get("rider_id");
            let rider_name: String = row.get("rider_name");
            let bid: Uuid = row.get("branch_id");
            let exp_dec: Decimal = row.get("total_expected");
            let col_dec: Decimal = row.get("total_collected");
            let dep_dec: Decimal = row.get("total_deposited");
            let var_dec: Decimal = row.get("total_variance");
            let session_count: i64 = row.get("session_count");
            let unresolved_sessions: i64 = row.get("unresolved_sessions");

            items.push(VarianceReportItem {
                rider_id: RiderId::from(rider_id),
                rider_name,
                branch_id: BranchId::from(bid),
                total_expected: Money::from_decimal(exp_dec),
                total_collected: Money::from_decimal(col_dec),
                total_deposited: Money::from_decimal(dep_dec),
                total_variance: Money::from_decimal(var_dec),
                session_count,
                unresolved_sessions,
            });
        }

        Ok(VarianceReportDto {
            start_date: start_date.to_string(),
            end_date: end_date.to_string(),
            branch_id,
            items,
        })
    }

    // --------------------------------------------------------------------------------------------
    // Public Customer Tracking (Zero PII, Doc 12 §8, §10)
    // --------------------------------------------------------------------------------------------

    pub async fn get_public_tracking(
        &self,
        tracking_token: &str,
    ) -> Result<PublicTrackingDto, FulfilmentError> {
        let row = sqlx::query(
            "SELECT d.order_id, d.status::text as status, d.assigned_at, d.picked_up_at, d.delivered_at,
                    b.name as branch_name, o.created_at as order_created_at
             FROM deliveries d
             LEFT JOIN branches b ON b.id = d.branch_id
             LEFT JOIN orders o ON o.id = d.order_id
             WHERE d.tracking_token = $1"
        )
        .bind(tracking_token)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FulfilmentError::NotFound("Tracking token not found".into()))?;

        let order_id: Uuid = row.get("order_id");
        let status_str: String = row.get("status");
        let branch_name: Option<String> = row.get("branch_name");
        let assigned_at = row.get("assigned_at");
        let picked_up_at = row.get("picked_up_at");
        let delivered_at = row.get("delivered_at");
        let order_created_at: DateTime<Utc> = row.get("order_created_at");

        let status = status_str.parse().unwrap_or(DeliveryStatus::Unassigned);
        let masked_ref = format!("ORD-{}", &order_id.to_string()[..8]);

        Ok(PublicTrackingDto {
            order_ref: masked_ref,
            status,
            branch_name: branch_name.unwrap_or_else(|| "Shifa Pharmacy".into()),
            estimated_delivery_time: Some(order_created_at + Duration::minutes(45)),
            assigned_at,
            picked_up_at,
            delivered_at,
        })
    }

    // --------------------------------------------------------------------------------------------
    // Helpers & Row Mappers
    // --------------------------------------------------------------------------------------------

    async fn verify_rider_or_admin(
        &self,
        ctx: &TenantContext,
        rider_id: RiderId,
    ) -> Result<(), FulfilmentError> {
        if ctx
            .role_names()
            .iter()
            .any(|r| r == "SUPER_ADMIN" || r == "BRANCH_MANAGER" || r == "SYSTEM")
        {
            return Ok(());
        }

        let caller_rider = self.get_rider_by_user_id(ctx, ctx.user_id()).await?;
        if let Some(r) = caller_rider {
            if r.id == rider_id {
                return Ok(());
            }
        }

        Err(FulfilmentError::Forbidden(
            "Action scoped to assigned rider".into(),
        ))
    }

    fn map_rider_row(&self, row: sqlx::postgres::PgRow) -> Result<RiderDto, FulfilmentError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let bid: Uuid = row.get("branch_id");
        let uid: Uuid = row.get("user_id");
        let vehicle_type: String = row.get("vehicle_type");
        let cnic: String = row.get("cnic");
        let licence_no: String = row.get("licence_no");
        let status_str: String = row.get("status");
        let on_shift: bool = row.get("on_shift");
        let decline_count: i32 = row.get("decline_count");
        let shift_started_at = row.get("shift_started_at");
        let shift_ended_at = row.get("shift_ended_at");
        let created_at = row.get("created_at");
        let updated_at = row.get("updated_at");

        let status = status_str.parse().unwrap_or(RiderStatus::Available);

        Ok(RiderDto {
            id: RiderId::from(id),
            tenant_id: TenantId::from(tid),
            branch_id: BranchId::from(bid),
            user_id: UserId::from(uid),
            vehicle_type,
            cnic,
            licence_no,
            status,
            on_shift,
            decline_count,
            shift_started_at,
            shift_ended_at,
            created_at,
            updated_at,
        })
    }

    fn map_delivery_row(&self, row: sqlx::postgres::PgRow) -> Result<DeliveryDto, FulfilmentError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let bid: Option<Uuid> = row.get("branch_id");
        let oid: Uuid = row.get("order_id");
        let rid: Option<Uuid> = row.get("rider_id");
        let status_str: String = row.get("status");
        let assigned_at = row.get("assigned_at");
        let accepted_at = row.get("accepted_at");
        let picked_up_at = row.get("picked_up_at");
        let in_transit_at = row.get("in_transit_at");
        let delivered_at = row.get("delivered_at");
        let failed_reason: Option<String> = row.get("failed_reason");
        let decline_reason: Option<String> = row.get("decline_reason");
        let pod_image_object_key: Option<String> = row.get("pod_image_object_key");
        let pod_signature_object_key: Option<String> = row.get("pod_signature_object_key");
        let recipient_name: Option<String> = row.get("recipient_name");
        let recipient_cnic_last4: Option<String> = row.get("recipient_cnic_last4");
        let prescription_collected: bool = row.get("prescription_collected");
        let cash_collected_dec: Option<Decimal> = row.get("cash_collected");
        let reattempt_count: i32 = row.get("reattempt_count");
        let tracking_token: String = row.get("tracking_token");
        let gps_denied_flag: bool = row.get("gps_denied_flag");
        let distance_km_dec: Option<Decimal> = row.get("distance_km");
        let created_at = row.get("created_at");
        let updated_at = row.get("updated_at");

        let status = status_str.parse().unwrap_or(DeliveryStatus::Unassigned);

        Ok(DeliveryDto {
            id: DeliveryId::from(id),
            tenant_id: TenantId::from(tid),
            branch_id: bid.map(BranchId::from),
            order_id: OrderId::from(oid),
            rider_id: rid.map(RiderId::from),
            status,
            assigned_at,
            accepted_at,
            picked_up_at,
            in_transit_at,
            delivered_at,
            failed_reason,
            decline_reason,
            pod_image_object_key,
            pod_signature_object_key,
            recipient_name,
            recipient_cnic_last4,
            prescription_collected,
            cash_collected: cash_collected_dec.map(Money::from_decimal),
            reattempt_count,
            tracking_token,
            gps_denied_flag,
            distance_km: distance_km_dec.map(|d| {
                use rust_decimal::prelude::ToPrimitive;
                d.to_f64().unwrap_or(0.0)
            }),
            created_at,
            updated_at,
        })
    }

    fn map_cash_session_row(
        &self,
        row: sqlx::postgres::PgRow,
    ) -> Result<RiderCashSessionDto, FulfilmentError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let rid: Uuid = row.get("rider_id");
        let bid: Option<Uuid> = row.get("branch_id");
        let status_str: String = row.get("status");
        let opened_at = row.get("opened_at");
        let closed_at = row.get("closed_at");
        let exp_dec: Decimal = row.get("expected_amount");
        let col_dec: Decimal = row.get("collected_amount");
        let dep_dec: Decimal = row.get("deposited_amount");
        let var_dec: Decimal = row.get("variance");
        let rec_by: Option<Uuid> = row.get("reconciled_by");
        let note: Option<String> = row.get("note");

        let status = status_str.parse().unwrap_or(CashSessionStatus::Open);

        Ok(RiderCashSessionDto {
            id: RiderCashSessionId::from(id),
            tenant_id: TenantId::from(tid),
            rider_id: RiderId::from(rid),
            branch_id: bid.map(BranchId::from),
            status,
            opened_at,
            closed_at,
            expected_amount: Money::from_decimal(exp_dec),
            collected_amount: Money::from_decimal(col_dec),
            deposited_amount: Money::from_decimal(dep_dec),
            variance: Money::from_decimal(var_dec),
            reconciled_by: rec_by.map(UserId::from),
            note,
        })
    }

    fn map_picking_list_row(
        &self,
        row: sqlx::postgres::PgRow,
    ) -> Result<PickingListDto, FulfilmentError> {
        let id: Uuid = row.get("id");
        let tid: Uuid = row.get("tenant_id");
        let bid: Uuid = row.get("branch_id");
        let oid: Uuid = row.get("order_id");
        let status_str: String = row.get("status");
        let items: serde_json::Value = row.get("items");
        let picked_by: Option<Uuid> = row.get("picked_by");
        let completed_at = row.get("completed_at");
        let created_at = row.get("created_at");
        let updated_at = row.get("updated_at");

        let status = status_str.parse().unwrap_or(PickingListStatus::Pending);

        Ok(PickingListDto {
            id: PickingListId::from(id),
            tenant_id: TenantId::from(tid),
            branch_id: BranchId::from(bid),
            order_id: OrderId::from(oid),
            status,
            items,
            picked_by: picked_by.map(UserId::from),
            completed_at,
            created_at,
            updated_at,
        })
    }

    async fn write_audit_log(
        &self,
        ctx: &TenantContext,
        target_id: Uuid,
        action: &str,
        details: serde_json::Value,
    ) -> Result<(), FulfilmentError> {
        let audit_id = Uuid::now_v7();
        let user_id = ctx.user_id().0;

        sqlx::query(
            "INSERT INTO audit_log (id, tenant_id, actor_id, actor_type, entity_type, entity_id, action, after, ip)
             VALUES ($1, $2, $3, 'USER', 'FULFILMENT', $4, $5, $6, '127.0.0.1')"
        )
        .bind(audit_id)
        .bind(ctx.tenant_id().0)
        .bind(user_id)
        .bind(target_id)
        .bind(action)
        .bind(&details)
        .execute(&self.pool)
        .await
        .ok();

        Ok(())
    }
}
