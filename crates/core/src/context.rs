use crate::error::CoreError;
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
    pub role_names: Vec<String>,
}

impl TenantContext {
    /// Construct a TenantContext strictly from authenticated claims.
    pub fn from_claims(
        tenant_id: TenantId,
        user_id: UserId,
        branch_ids: Vec<BranchId>,
        permissions: HashSet<String>,
        role_names: Vec<String>,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            branch_ids,
            permissions,
            role_names,
        }
    }

    /// Check if the authenticated context holds a specific permission key.
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    /// Assert that the authenticated context holds a specific permission key.
    pub fn require(&self, permission: &str) -> Result<(), CoreError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(CoreError::PermissionDenied(permission.to_string()))
        }
    }

    /// Check if the authenticated context can operate on a target branch.
    /// An empty `branch_ids` vector denotes organization-wide administrative scope.
    pub fn can_act_on_branch(&self, branch_id: BranchId) -> bool {
        self.branch_ids.is_empty() || self.branch_ids.contains(&branch_id)
    }

    /// Assert that the authenticated context can operate on a target branch.
    pub fn require_branch(&self, branch_id: BranchId) -> Result<(), CoreError> {
        if self.can_act_on_branch(branch_id) {
            Ok(())
        } else {
            Err(CoreError::BranchAccessDenied(branch_id.to_string()))
        }
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

        let ctx = TenantContext::from_claims(
            tenant_id,
            user_id,
            vec![branch_a],
            perms,
            vec!["PHARMACIST".to_string()],
        );

        assert_eq!(ctx.tenant_id, tenant_id);
        assert_eq!(ctx.user_id, user_id);
        assert!(ctx.has_permission("rx.approve"));
        assert!(ctx.has_permission("payment.approve"));
        assert!(!ctx.has_permission("admin.settings"));
        assert!(ctx.require("rx.approve").is_ok());
        assert!(ctx.require("admin.settings").is_err());

        assert!(ctx.can_act_on_branch(branch_a));
        assert!(!ctx.can_act_on_branch(branch_b));
        assert!(ctx.require_branch(branch_a).is_ok());
        assert!(ctx.require_branch(branch_b).is_err());
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
            vec!["SUPER_ADMIN".to_string()],
        );

        assert!(ctx.can_act_on_branch(branch_any));
        assert!(ctx.require_branch(branch_any).is_ok());
    }
}
