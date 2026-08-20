use crate::error::OrderError;
use crate::models::*;
use crate::numbering::generate_order_number;
use crate::pricing::{calculate_line_total, calculate_order_total, validate_item_price};
use crate::state_machine::{can_transition, OrderStatus};
use rust_decimal::Decimal;
use shifa_core::context::TenantContext;
use shifa_core::id::{BranchId, CustomerId, OrderId, ProductId, TenantId};
use shifa_core::money::Money;
use shifa_inventory::reservations::{
    release_expired_reservations, reserve_stock, ReserveStockParams,
};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OrderService {
    pool: PgPool,
}

impl OrderService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create new draft order
    pub async fn create_draft_order(
        &self,
        ctx: &TenantContext,
        req: CreateDraftOrderRequest,
    ) -> Result<OrderDto, OrderError> {
        let order_id = OrderId::new();
        let branch_code = "MAIN";
        let order_no = generate_order_number(&self.pool, ctx.tenant_id.0, branch_code)
            .await
            .map_err(OrderError::Sqlx)?;

        let payment_method = req.payment_method.unwrap_or_else(|| "COD".to_string());

        sqlx::query(
            "INSERT INTO orders (id, tenant_id, order_no, customer_id, branch_id, status, subtotal, discount, delivery_fee, tax, total, payment_method, payment_status, fulfilment_type, is_prescription_only)
             VALUES ($1, $2, $3, $4, $5, 'Draft', 0.00, 0.00, 0.00, 0.00, 0.00, $6, 'PENDING', 'DELIVERY', false)"
        )
        .bind(order_id.0)
        .bind(ctx.tenant_id.0)
        .bind(&order_no)
        .bind(req.customer_id.0)
        .bind(req.branch_id.map(|b| b.0))
        .bind(&payment_method)
        .execute(&self.pool)
        .await?;

        // Write order creation event and audit log
        sqlx::query(
            "INSERT INTO order_events (id, tenant_id, order_id, to_status, actor_id, reason)
             VALUES ($1, $2, $3, 'Draft', $4, 'Created draft order')",
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id.0)
        .bind(order_id.0)
        .bind(ctx.user_id.0)
        .execute(&self.pool)
        .await?;

        self.get_order(ctx, order_id).await
    }

    /// Add line item to order with MRP validation and snapshotting per Doc 10 §8.
    pub async fn add_order_item(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
        req: AddOrderItemRequest,
    ) -> Result<OrderDto, OrderError> {
        let product_row = sqlx::query(
            "SELECT brand_name, mrp, is_prescription_only, is_refrigerated
             FROM products
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(ctx.tenant_id.0)
        .bind(req.product_id.0)
        .fetch_optional(&self.pool)
        .await?;

        let (_brand_name, mrp_dec, is_rx, _is_refrig) = match product_row {
            Some(r) => (
                r.get::<String, _>("brand_name"),
                r.get::<Decimal, _>("mrp"),
                r.get::<bool, _>("is_prescription_only"),
                r.get::<bool, _>("is_refrigerated"),
            ),
            None => return Err(OrderError::ItemNotFound(req.product_id.0)),
        };

        let mrp = Money::from_decimal(mrp_dec);
        let unit_price = req.unit_price.unwrap_or(mrp);
        let line_discount = req.discount.unwrap_or_else(Money::zero);

        // Enforce MRP hard block
        validate_item_price(unit_price, mrp)?;

        let line_total = calculate_line_total(req.qty, unit_price, line_discount);
        let item_id = Uuid::now_v7();

        sqlx::query(
            "INSERT INTO order_items (id, tenant_id, order_id, product_id, quantity, unit_price, mrp_at_sale, line_discount, total_price)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(item_id)
        .bind(ctx.tenant_id.0)
        .bind(order_id.0)
        .bind(req.product_id.0)
        .bind(req.qty)
        .bind(unit_price.amount())
        .bind(mrp.amount())
        .bind(line_discount.amount())
        .bind(line_total.amount())
        .execute(&self.pool)
        .await?;

        // Update order subtotal, total, and Rx flag
        self.recalculate_order_totals(ctx, order_id, is_rx).await?;

        self.get_order(ctx, order_id).await
    }

    /// Confirm cart: transitions Draft to AwaitingRx or AwaitingPayment per Doc 10 §4.1.
    /// Invariant: Orders with Rx items must transition to AwaitingRx, never directly to AwaitingPayment.
    pub async fn confirm_cart(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
    ) -> Result<OrderDto, OrderError> {
        let order = self.get_order(ctx, order_id).await?;

        if order.status != OrderStatus::Draft {
            return Err(OrderError::InvalidTransition {
                from: order.status.to_string(),
                to: "CartConfirmed".into(),
            });
        }

        let target_status = if order.is_rx_linked {
            OrderStatus::AwaitingRx
        } else {
            OrderStatus::AwaitingPayment
        };

        self.transition_order(
            ctx,
            order_id,
            TransitionOrderRequest {
                to_status: target_status,
                reason: Some("Cart confirmed by customer / agent".into()),
            },
        )
        .await
    }

    /// Transition order status atomically writing order_events and audit_log (Invariant I-9).
    pub async fn transition_order(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
        req: TransitionOrderRequest,
    ) -> Result<OrderDto, OrderError> {
        let order = self.get_order(ctx, order_id).await?;
        let current_status = order.status;
        let target_status = req.to_status;

        // 1. Validate transition predicate
        if !can_transition(current_status, target_status) {
            return Err(OrderError::InvalidTransition {
                from: current_status.to_string(),
                to: target_status.to_string(),
            });
        }

        // 2. Rx branching guard: cannot skip to AwaitingPayment if Rx items present
        if order.is_rx_linked
            && current_status == OrderStatus::CartConfirmed
            && target_status == OrderStatus::AwaitingPayment
        {
            return Err(OrderError::RxItemRequiresReview);
        }

        // 3. Atomic transition in a database transaction
        let mut tx = self.pool.begin().await.map_err(OrderError::Sqlx)?;

        sqlx::query(
            "UPDATE orders
             SET status = $1, updated_at = now()
             WHERE tenant_id = $2 AND id = $3",
        )
        .bind(target_status.to_string())
        .bind(ctx.tenant_id.0)
        .bind(order_id.0)
        .execute(&mut *tx)
        .await
        .map_err(OrderError::Sqlx)?;

        sqlx::query(
            "INSERT INTO order_events (id, tenant_id, order_id, from_status, to_status, actor_id, reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(Uuid::now_v7())
        .bind(ctx.tenant_id.0)
        .bind(order_id.0)
        .bind(current_status.to_string())
        .bind(target_status.to_string())
        .bind(ctx.user_id.0)
        .bind(&req.reason)
        .execute(&mut *tx)
        .await
        .map_err(OrderError::Sqlx)?;

        // Invariant I-9: Audit log must succeed or whole transition rolls back
        sqlx::query(
            "INSERT INTO audit_log (tenant_id, actor_id, actor_type, entity_type, entity_id, action, before, after, reason)
             VALUES ($1, $2, 'USER', 'ORDER', $3, 'TRANSITION_STATUS', $4, $5, $6)"
        )
        .bind(ctx.tenant_id.0)
        .bind(ctx.user_id.0)
        .bind(order_id.0)
        .bind(serde_json::json!({"status": current_status.to_string()}))
        .bind(serde_json::json!({"status": target_status.to_string()}))
        .bind(req.reason.as_deref().unwrap_or("Order state transition"))
        .execute(&mut *tx)
        .await
        .map_err(OrderError::Sqlx)?;

        // 4. Side effects: on Confirmed, reserve stock with TTL
        if target_status == OrderStatus::Confirmed {
            if let Some(branch_id) = order.branch_id {
                for item in &order.items {
                    let candidate_batch = sqlx::query(
                        "SELECT batch_id FROM stock_current
                         WHERE tenant_id = $1 AND branch_id = $2 AND product_id = $3 AND qty >= $4
                         LIMIT 1",
                    )
                    .bind(ctx.tenant_id.0)
                    .bind(branch_id.0)
                    .bind(item.product_id.0)
                    .bind(item.qty)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(OrderError::Sqlx)?;

                    if let Some(r) = candidate_batch {
                        let b_id: Uuid = r.get("batch_id");
                        // Reserve stock
                        let _ = reserve_stock(
                            ctx,
                            &self.pool,
                            ReserveStockParams {
                                order_id: order_id.0,
                                branch_id,
                                product_id: item.product_id,
                                batch_id: shifa_core::id::BatchId::from(b_id),
                                qty: item.qty,
                                ttl_minutes: 120, // 2 hours COD
                            },
                        )
                        .await;
                    }
                }
            }
        }

        // On Cancelled or Rejected, release reservations
        if target_status == OrderStatus::Cancelled
            || target_status == OrderStatus::RxRejected
            || target_status == OrderStatus::PaymentRejected
        {
            let _ = release_expired_reservations(&self.pool).await;
        }

        tx.commit().await.map_err(OrderError::Sqlx)?;

        self.get_order(ctx, order_id).await
    }

    /// Process returned items with pharmacist certification checks per Doc 10 §9.
    pub async fn return_items(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
        req: Vec<ReturnItemRequest>,
    ) -> Result<(), OrderError> {
        ctx.require("order.edit")
            .map_err(|e| OrderError::Unauthorized(e.to_string()))?;

        for item in req {
            // Check if item is medicine / prescription
            let item_row = sqlx::query(
                "SELECT oi.product_id, p.is_prescription_only, p.is_refrigerated
                 FROM order_items oi
                 JOIN products p ON p.id = oi.product_id AND p.tenant_id = oi.tenant_id
                 WHERE oi.tenant_id = $1 AND oi.order_id = $2 AND oi.id = $3",
            )
            .bind(ctx.tenant_id.0)
            .bind(order_id.0)
            .bind(item.item_id)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(r) = item_row {
                let is_rx: bool = r.get("is_prescription_only");
                let is_refrig: bool = r.get("is_refrigerated");

                if is_refrig && item.is_safe_to_restock {
                    return Err(OrderError::ColdChainRestockForbidden);
                }

                if is_rx && item.is_safe_to_restock && !item.pharmacist_certified {
                    return Err(OrderError::RestockRequiresCertification);
                }
            }
        }

        Ok(())
    }

    /// Get order by ID with line items
    pub async fn get_order(
        &self,
        ctx: &TenantContext,
        id: OrderId,
    ) -> Result<OrderDto, OrderError> {
        let order_row = sqlx::query(
            "SELECT id, tenant_id, order_no, customer_id, branch_id, status, subtotal, discount, delivery_fee, tax, total, payment_method, payment_status, is_prescription_only, created_at
             FROM orders
             WHERE tenant_id = $1 AND id = $2"
        )
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .fetch_optional(&self.pool)
        .await?;

        let row = match order_row {
            Some(r) => r,
            None => return Err(OrderError::NotFound(id)),
        };

        let items_rows = sqlx::query(
            "SELECT oi.id, oi.product_id, p.brand_name, oi.quantity, oi.unit_price, oi.mrp_at_sale, oi.line_discount, oi.total_price, p.is_prescription_only, p.is_refrigerated
             FROM order_items oi
             JOIN products p ON p.id = oi.product_id AND p.tenant_id = oi.tenant_id
             WHERE oi.tenant_id = $1 AND oi.order_id = $2"
        )
        .bind(ctx.tenant_id.0)
        .bind(id.0)
        .fetch_all(&self.pool)
        .await?;

        let items = items_rows
            .into_iter()
            .map(|i| OrderItemDto {
                id: i.get("id"),
                product_id: ProductId::from(i.get::<Uuid, _>("product_id")),
                product_name: i.get("brand_name"),
                qty: i.get("quantity"),
                unit_price: Money::from_decimal(i.get("unit_price")),
                mrp_at_sale: Money::from_decimal(i.get("mrp_at_sale")),
                line_discount: Money::from_decimal(i.get("line_discount")),
                line_total: Money::from_decimal(i.get("total_price")),
                is_prescription_only: i.get("is_prescription_only"),
                is_refrigerated: i.get("is_refrigerated"),
            })
            .collect();

        let status_str: String = row.get("status");
        let status = OrderStatus::from_str(&status_str).unwrap_or(OrderStatus::Draft);

        Ok(OrderDto {
            id,
            tenant_id: TenantId::from(row.get::<Uuid, _>("tenant_id")),
            order_no: row.get("order_no"),
            customer_id: CustomerId::from(row.get::<Uuid, _>("customer_id")),
            branch_id: row.get::<Option<Uuid>, _>("branch_id").map(BranchId::from),
            status,
            is_rx_linked: row.get("is_prescription_only"),
            subtotal: Money::from_decimal(row.get("subtotal")),
            discount: Money::from_decimal(row.get("discount")),
            delivery_fee: Money::from_decimal(row.get("delivery_fee")),
            tax_amount: Money::from_decimal(row.get("tax")),
            total: Money::from_decimal(row.get("total")),
            payment_method: row.get("payment_method"),
            payment_status: row.get("payment_status"),
            items,
            created_at: row.get("created_at"),
        })
    }

    /// List orders
    pub async fn list_orders(
        &self,
        ctx: &TenantContext,
        branch_id: Option<BranchId>,
        status: Option<&str>,
    ) -> Result<Vec<OrderDto>, OrderError> {
        let rows = sqlx::query(
            "SELECT id FROM orders
             WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR branch_id = $2)
               AND ($3::text IS NULL OR status = $3)
             ORDER BY created_at DESC",
        )
        .bind(ctx.tenant_id.0)
        .bind(branch_id.map(|b| b.0))
        .bind(status)
        .fetch_all(&self.pool)
        .await?;

        let mut orders = Vec::new();
        for r in rows {
            let id: Uuid = r.get("id");
            if let Ok(order) = self.get_order(ctx, OrderId::from(id)).await {
                orders.push(order);
            }
        }

        Ok(orders)
    }

    async fn recalculate_order_totals(
        &self,
        ctx: &TenantContext,
        order_id: OrderId,
        has_new_rx_item: bool,
    ) -> Result<(), OrderError> {
        let subtotal_row = sqlx::query(
            "SELECT COALESCE(SUM(total_price), 0)::numeric as subtotal
             FROM order_items
             WHERE tenant_id = $1 AND order_id = $2",
        )
        .bind(ctx.tenant_id.0)
        .bind(order_id.0)
        .fetch_one(&self.pool)
        .await?;

        let subtotal_dec: Decimal = subtotal_row.get("subtotal");
        let subtotal = Money::from_decimal(subtotal_dec);
        let delivery_fee = Money::from_decimal(Decimal::from(100)); // Default Rs 100 delivery fee
        let total = calculate_order_total(subtotal, Money::zero(), delivery_fee, Money::zero());

        sqlx::query(
            "UPDATE orders
             SET subtotal = $1, delivery_fee = $2, total = $3,
                 is_prescription_only = is_prescription_only OR $4, updated_at = now()
             WHERE tenant_id = $5 AND id = $6",
        )
        .bind(subtotal.amount())
        .bind(delivery_fee.amount())
        .bind(total.amount())
        .bind(has_new_rx_item)
        .bind(ctx.tenant_id.0)
        .bind(order_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
