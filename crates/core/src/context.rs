use crate::error::CoreError;
use crate::id::{BranchId, TenantId, UserId};
use std::collections::HashSet;

/// Authenticated tenant context extracted strictly from verified JWT/session claims.
///
/// Fields are private so callers cannot assemble a context from request body/path/query.
/// Construction is only via [`TenantContext::from_verified_claims`], which the identity
/// layer must call after JWT/session verification — never from handler-supplied IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantContext {
    tenant_id: TenantId,
    user_id: UserId,
    branch_ids: Vec<BranchId>,
    permissions: HashSet<String>,
    role_names: Vec<String>,
    org_wide_branch_access: bool,
}

impl TenantContext {
    /// Mint a context after JWT/session verification in the identity crate.
    ///
    /// `org_wide_branch_access` must be an explicit claim (e.g. SUPER_ADMIN).
    /// An empty `branch_ids` list never implies org-wide access.
    pub fn from_verified_claims(
        tenant_id: TenantId,
        user_id: UserId,
        branch_ids: Vec<BranchId>,
        permissions: HashSet<String>,
        role_names: Vec<String>,
        org_wide_branch_access: bool,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            branch_ids,
            permissions,
            role_names,
            org_wide_branch_access,
        }
    }

    pub fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn branch_ids(&self) -> &[BranchId] {
        &self.branch_ids
    }

    pub fn permissions(&self) -> &HashSet<String> {
        &self.permissions
    }

    pub fn role_names(&self) -> &[String] {
        &self.role_names
    }

    pub fn org_wide_branch_access(&self) -> bool {
        self.org_wide_branch_access
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(permission)
    }

    pub fn require(&self, permission: &str) -> Result<(), CoreError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(CoreError::PermissionDenied(permission.to_string()))
        }
    }

    /// Empty `branch_ids` means no branch access unless `org_wide_branch_access` is set.
    pub fn can_act_on_branch(&self, branch_id: BranchId) -> bool {
        self.org_wide_branch_access || self.branch_ids.contains(&branch_id)
    }

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

        let ctx = TenantContext::from_verified_claims(
            tenant_id,
            user_id,
            vec![branch_a],
            perms,
            vec!["PHARMACIST".to_string()],
            false,
        );

        assert_eq!(ctx.tenant_id(), tenant_id);
        assert_eq!(ctx.user_id(), user_id);
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
    fn empty_branch_ids_deny_all_without_org_wide_flag() {
        let branch_any = BranchId::new();
        let ctx = TenantContext::from_verified_claims(
            TenantId::new(),
            UserId::new(),
            vec![],
            HashSet::new(),
            vec!["CASHIER".to_string()],
            false,
        );
        assert!(!ctx.can_act_on_branch(branch_any));
        assert!(ctx.require_branch(branch_any).is_err());
    }

    #[test]
    fn org_wide_flag_allows_any_branch() {
        let branch_any = BranchId::new();
        let ctx = TenantContext::from_verified_claims(
            TenantId::new(),
            UserId::new(),
            vec![],
            HashSet::new(),
            vec!["SUPER_ADMIN".to_string()],
            true,
        );
        assert!(ctx.can_act_on_branch(branch_any));
        assert!(ctx.require_branch(branch_any).is_ok());
    }
}
