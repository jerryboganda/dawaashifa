use crate::id::{BranchId, TenantId, UserId};
use std::collections::HashSet;

/// Authenticated tenant context extracted strictly from verified JWT/session claims.
/// Invariant: TenantContext is constructed only from authenticated claims.
/// NOTE: Intentionally NOT deriving `serde::Deserialize` to prevent injection from request bodies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub branch_ids: Vec<BranchId>,
    pub permissions: HashSet<String>,
}

impl TenantContext {
    /// Construct a TenantContext strictly from authenticated claims.
    pub fn from_claims(
        tenant_id: TenantId,
        user_id: UserId,
        branch_ids: Vec<BranchId>,
        permissions: HashSet<String>,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            branch_ids,
            permissions,
        }
    }

    /// Check if the authenticated context holds a specific permission key.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if the authenticated context can operate on a target branch.
    /// An empty `branch_ids` vector denotes organization-wide administrative scope.
    pub fn can_access_branch(&self, branch_id: BranchId) -> bool {
        self.branch_ids.is_empty() || self.branch_ids.contains(&branch_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_context_permissions_and_branches() {
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let branch_a = BranchId::new();
        let branch_b = BranchId::new();

        let mut perms = HashSet::new();
        perms.insert("rx.approve".to_string());
        perms.insert("payment.approve".to_string());

        let ctx = TenantContext::from_claims(tenant_id, user_id, vec![branch_a], perms);

        assert_eq!(ctx.tenant_id, tenant_id);
        assert_eq!(ctx.user_id, user_id);
        assert!(ctx.has_permission("rx.approve"));
        assert!(ctx.has_permission("payment.approve"));
        assert!(!ctx.has_permission("admin.settings"));

        assert!(ctx.can_access_branch(branch_a));
        assert!(!ctx.can_access_branch(branch_b));
    }

    #[test]
    fn test_org_wide_admin_branch_access() {
        let tenant_id = TenantId::new();
        let user_id = UserId::new();
        let branch_any = BranchId::new();

        let ctx = TenantContext::from_claims(
            tenant_id,
            user_id,
            vec![], // Empty = all branches allowed
            HashSet::new(),
        );

        assert!(ctx.can_access_branch(branch_any));
    }
}
