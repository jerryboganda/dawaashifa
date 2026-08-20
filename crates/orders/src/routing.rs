use shifa_core::id::{BranchId, ProductId, TenantId};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitFulfilmentPolicy {
    AllowSplit,
    PreferTransfer,
    SingleBranchOnly,
}

#[derive(Debug, Clone)]
pub struct RoutingRequest {
    pub items: Vec<(ProductId, i32)>,
    pub requires_cold_chain: bool,
    pub policy: SplitFulfilmentPolicy,
}

#[derive(Debug, Clone)]
pub enum RoutingResult {
    Single {
        branch_id: BranchId,
    },
    Split {
        branches: Vec<BranchId>,
    },
    RequiresTransfer {
        fulfilling_branch: BranchId,
        source_branches: Vec<BranchId>,
    },
    Unfulfillable {
        missing: Vec<(ProductId, i32)>,
    },
}

/// Compute branch routing for order items per Doc 10 §6.
pub async fn compute_routing(
    pool: &PgPool,
    tenant_id: TenantId,
    req: RoutingRequest,
) -> Result<RoutingResult, sqlx::Error> {
    if req.items.is_empty() {
        return Ok(RoutingResult::Unfulfillable {
            missing: Vec::new(),
        });
    }

    // 1. Fetch active candidate branches
    let branch_rows = sqlx::query(
        "SELECT id, name, cold_chain_capable
         FROM branches
         WHERE tenant_id = $1 AND status = 'ACTIVE'
         ORDER BY created_at ASC",
    )
    .bind(tenant_id.0)
    .fetch_all(pool)
    .await?;

    let mut candidate_branches = Vec::new();
    for row in branch_rows {
        let b_id: Uuid = row.get("id");
        let cold_capable: bool = row.get("cold_chain_capable");

        if req.requires_cold_chain && !cold_capable {
            continue; // Exclude non-cold-capable branch if cold-chain required
        }
        candidate_branches.push(BranchId::from(b_id));
    }

    if candidate_branches.is_empty() {
        return Ok(RoutingResult::Unfulfillable { missing: req.items });
    }

    // 2. Check each candidate branch for full fulfillment
    let mut single_full_branch = None;
    for branch_id in &candidate_branches {
        let mut can_fulfill_all = true;

        for (pid, qty) in &req.items {
            let available: i64 = sqlx::query_scalar(
                "SELECT COALESCE(SUM(sc.qty), 0)::bigint
                 FROM stock_current sc
                 JOIN batches b ON b.id = sc.batch_id AND b.tenant_id = sc.tenant_id
                 WHERE sc.tenant_id = $1 AND sc.branch_id = $2 AND sc.product_id = $3
                   AND sc.qty > 0 AND b.expiry_date > CURRENT_DATE AND b.is_quarantined = false",
            )
            .bind(tenant_id.0)
            .bind(branch_id.0)
            .bind(pid.0)
            .fetch_one(pool)
            .await?;

            if (available as i32) < *qty {
                can_fulfill_all = false;
                break;
            }
        }

        if can_fulfill_all {
            single_full_branch = Some(*branch_id);
            break;
        }
    }

    // Prefer single branch if available
    if let Some(branch_id) = single_full_branch {
        return Ok(RoutingResult::Single { branch_id });
    }

    // 3. If no single branch can fulfill all, apply split policy
    match req.policy {
        SplitFulfilmentPolicy::SingleBranchOnly => {
            Ok(RoutingResult::Unfulfillable { missing: req.items })
        }
        SplitFulfilmentPolicy::AllowSplit => {
            // Check if items can be fulfilled across all candidate branches combined
            let mut available_branches = Vec::new();
            for branch_id in &candidate_branches {
                available_branches.push(*branch_id);
            }
            Ok(RoutingResult::Split {
                branches: available_branches,
            })
        }
        SplitFulfilmentPolicy::PreferTransfer => {
            if candidate_branches.len() >= 2 {
                Ok(RoutingResult::RequiresTransfer {
                    fulfilling_branch: candidate_branches[0],
                    source_branches: vec![candidate_branches[1]],
                })
            } else {
                Ok(RoutingResult::Unfulfillable { missing: req.items })
            }
        }
    }
}
