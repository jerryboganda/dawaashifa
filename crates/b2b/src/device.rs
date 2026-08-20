use chrono::Utc;
use shifa_core::context::TenantContext;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::error::B2bError;
use crate::models::{DeviceUnitDto, RecallQueryResponse, RegisterDeviceRequest};

pub struct DeviceTraceability;

impl DeviceTraceability {
    /// Registers a single implantable medical device unit (Doc 14 §11)
    pub async fn register_device(
        ctx: &TenantContext,
        req: RegisterDeviceRequest,
        pool: &PgPool,
    ) -> Result<DeviceUnitDto, B2bError> {
        let device_id = Uuid::now_v7();

        let exists = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM device_units WHERE tenant_id = $1 AND serial_no = $2",
        )
        .bind(ctx.tenant_id().0)
        .bind(&req.serial_no)
        .fetch_one(pool)
        .await?;

        if exists > 0 {
            return Err(B2bError::DeviceSerialDuplicate(req.serial_no));
        }

        sqlx::query(
            "INSERT INTO device_units (id, tenant_id, product_id, batch_id, serial_no, udi, status, location_type, location_id)
             VALUES ($1, $2, $3, $4, $5, $6, 'IN_STOCK', $7, $8)"
        )
        .bind(device_id)
        .bind(ctx.tenant_id().0)
        .bind(req.product_id)
        .bind(req.batch_id)
        .bind(&req.serial_no)
        .bind(req.udi.as_deref())
        .bind(req.location_type.as_deref().unwrap_or("WAREHOUSE"))
        .bind(req.location_id)
        .execute(pool)
        .await?;

        Ok(DeviceUnitDto {
            id: device_id,
            tenant_id: ctx.tenant_id().0,
            product_id: req.product_id,
            batch_id: req.batch_id,
            serial_no: req.serial_no,
            udi: req.udi,
            status: "IN_STOCK".to_string(),
            location_type: req.location_type.unwrap_or_else(|| "WAREHOUSE".to_string()),
            location_id: req.location_id,
            implanted_at: None,
            patient_ref: None,
            surgeon_name: None,
            order_id: None,
            created_at: Utc::now(),
        })
    }

    /// First-class manufacturer recall query (Doc 14 §11)
    pub async fn query_recall(
        ctx: &TenantContext,
        product_id: Option<Uuid>,
        batch_id: Option<Uuid>,
        pool: &PgPool,
    ) -> Result<RecallQueryResponse, B2bError> {
        let rows = sqlx::query(
            "SELECT id, product_id, batch_id, serial_no, udi, status, location_type, location_id, implanted_at, patient_ref, surgeon_name, order_id, created_at
             FROM device_units
             WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR product_id = $2)
               AND ($3::uuid IS NULL OR batch_id = $3)"
        )
        .bind(ctx.tenant_id().0)
        .bind(product_id)
        .bind(batch_id)
        .fetch_all(pool)
        .await?;

        let mut units = Vec::new();
        for r in rows {
            units.push(DeviceUnitDto {
                id: r.get("id"),
                tenant_id: ctx.tenant_id().0,
                product_id: r.get("product_id"),
                batch_id: r.get("batch_id"),
                serial_no: r.get("serial_no"),
                udi: r.get("udi"),
                status: r.get("status"),
                location_type: r.get("location_type"),
                location_id: r.get("location_id"),
                implanted_at: r.get("implanted_at"),
                patient_ref: r.get("patient_ref"),
                surgeon_name: r.get("surgeon_name"),
                order_id: r.get("order_id"),
                created_at: r.get("created_at"),
            });
        }

        Ok(RecallQueryResponse {
            product_id,
            batch_id,
            affected_units_count: units.len(),
            units,
        })
    }
}
