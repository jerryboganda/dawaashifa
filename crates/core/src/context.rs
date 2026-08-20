use crate::error::CoreError;
use crate::id::{BranchId, TenantId, UserId};
use std::collections::HashSet;

/// Authenticated tenant context. Fields are private.
///
/// Production minting: [`TenantContext::from_authenticated_session`] derives
/// org-wide branch access from `role_names` (SUPER_ADMIN only). Never pass a
/// caller-supplied org-wide boolean. HTTP handlers must not call this — they
/// receive context from the JWT extractor via identity.
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
    /// Build context after JWT/session verification.
    ///
    /// `org_wide_branch_access` is derived: true iff `role_names` contains
    /// `SUPER_ADMIN`. Empty `branch_ids` never implies org-wide access.
    pub fn from_authenticated_session(
        tenant_id: TenantId,
        user_id: UserId,
        branch_ids: Vec<BranchId>,
        permissions: HashSet<String>,
        role_names: Vec<String>,
    ) -> Self {
        let org_wide_branch_access = role_names.iter().any(|r| r == "SUPER_ADMIN");
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

    /// Empty `branch_ids` means no branch access unless SUPER_ADMIN.
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

        let ctx = TenantContext::from_authenticated_session(
            tenant_id,
            user_id,
            vec![branch_a],
            perms,
            vec!["PHARMACIST".to_string()],
        );

        assert_eq!(ctx.tenant_id(), tenant_id);
        assert_eq!(ctx.user_id(), user_id);
        assert!(ctx.has_permission("rx.approve"));
        assert!(!ctx.org_wide_branch_access());
        assert!(ctx.can_act_on_branch(branch_a));
        assert!(!ctx.can_act_on_branch(branch_b));
        assert!(ctx.require_branch(branch_a).is_ok());
        assert!(ctx.require_branch(branch_b).is_err());
    }

    #[test]
    fn empty_branch_ids_denies_access() {
        let branch_any = BranchId::new();
        let ctx = TenantContext::from_authenticated_session(
            TenantId::new(),
            UserId::new(),
            vec![],
            HashSet::new(),
            vec!["CASHIER".to_string()],
        );
        assert!(!ctx.org_wide_branch_access());
        assert!(!ctx.can_act_on_branch(branch_any));
        assert!(ctx.require_branch(branch_any).is_err());
    }

    #[test]
    fn org_wide_flag_grants_access_only_for_super_admin() {
        let branch_any = BranchId::new();
        let admin = TenantContext::from_authenticated_session(
            TenantId::new(),
            UserId::new(),
            vec![],
            HashSet::new(),
            vec!["SUPER_ADMIN".to_string()],
        );
        assert!(admin.org_wide_branch_access());
        assert!(admin.can_act_on_branch(branch_any));

        let not_admin = TenantContext::from_authenticated_session(
            TenantId::new(),
            UserId::new(),
            vec![],
            HashSet::new(),
            vec!["CASHIER".to_string()],
        );
        assert!(!not_admin.org_wide_branch_access());
        assert!(!not_admin.can_act_on_branch(branch_any));
    }
}
