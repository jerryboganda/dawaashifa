# DOC 04 — IDENTITY, RBAC, BRANCHES & SESSIONS

**Agent:** Backend (Copilot)
**Depends on:** Doc 01
**Produces:** `crates/identity`, auth middleware in `crates/api`
**Branch:** `feat/04-identity-rbac`

---

## 1. Objective

Authentication, authorisation, branch scoping, and the `TenantContext` extractor every other spec depends on.

## 2. In scope

- Password auth (argon2id), JWT access + refresh, session revocation
- Role and permission model with seeded system roles
- Branch scoping — a user acts only on assigned branches
- `TenantContext` Axum extractor
- `require_permission!` middleware
- User CRUD, role assignment, branch assignment endpoints
- Audit logging of every auth event

## 3. Out of scope — do NOT build

- SSO / OAuth / social login
- Customer authentication (customers are identified by WhatsApp number, not logged in)
- Rider auth (Doc 12 — riders get a separate scoped token)
- Frontend login screens (Doc 16)
- Password reset email delivery (stub the sender; wire it in Doc 17)

## 4. Roles — seeded as `is_system = true`

| Role | Key permissions |
|---|---|
| `SUPER_ADMIN` | everything, all branches, tenant settings |
| `OPERATIONS_HEAD` | all branches, all operational permissions, no tenant settings |
| `BRANCH_MANAGER` | assigned branches: inbox, orders, inventory, payment approval, reply override |
| `PHARMACIST` | **`rx.approve`**, inbox read, product read, order read |
| `PHARMACY_ASSISTANT` | inbox, orders, inventory read — no approvals |
| `INVENTORY_CONTROLLER` | stock receipt, adjustment, transfer, batch management |
| `ACCOUNTANT` | payments, invoices, reconciliation, reports — read-only on orders |
| `RIDER` | own deliveries only, via scoped token |
| `B2B_DESK` | quotes, hospital accounts, credit limits |
| `AUDITOR` | read-only across everything, including audit log |

## 5. Permission keys

```
rx.view  rx.approve  rx.reject
order.view  order.create  order.edit  order.cancel  order.refund
payment.view  payment.approve  payment.reject  payment.refund
inventory.view  inventory.receive  inventory.adjust  inventory.transfer
inbox.view  inbox.reply  inbox.override  inbox.assign
product.view  product.create  product.edit  product.price
branch.view  branch.create  branch.edit
user.view  user.create  user.edit  user.assign_role
report.view  report.export
audit.view
tenant.settings
b2b.quote  b2b.credit
```

**`rx.approve` belongs only to `PHARMACIST` and `SUPER_ADMIN`.** A test must assert no other seeded role holds it — this enforces invariant I-3 at the permission layer.

## 6. Contracts

```rust
pub struct TenantContext {
    pub tenant_id: TenantId,
    pub user_id: UserId,
    pub branch_ids: Vec<BranchId>,
    pub permissions: HashSet<String>,
    pub role_names: Vec<String>,
}

impl TenantContext {
    pub fn require(&self, perm: &str) -> Result<(), AuthError>;
    pub fn can_act_on_branch(&self, b: BranchId) -> bool;
    pub fn require_branch(&self, b: BranchId) -> Result<(), AuthError>;
}
```

Axum extractor: parse `Authorization: Bearer`, verify JWT, check session not revoked, load permissions (Redis-cached, 60s TTL), build context.

**`tenant_id` comes only from JWT claims. Never from a path, query, header, or body.** A handler that accepts `tenant_id` as a parameter is a security defect.

## 7. JWT

```json
{ "sub": "<user_id>", "tid": "<tenant_id>", "sid": "<session_id>",
  "roles": ["PHARMACIST"], "exp": 1234567890, "iat": 1234567890 }
```
- Access token: 15 minutes, HS256 (key from env, min 32 bytes)
- Refresh token: 30 days, opaque random, hashed in `sessions.token_hash`
- Rotate refresh on every use; reuse of a rotated token revokes the whole session family and raises a security alert

## 8. Endpoints

```
POST   /api/v1/auth/login              {phone|email, password} → tokens
POST   /api/v1/auth/refresh            {refresh_token} → tokens
POST   /api/v1/auth/logout             revoke current session
GET    /api/v1/auth/me                 current user, roles, branches, permissions
POST   /api/v1/auth/password/change    {current, new}

GET    /api/v1/users                   ?branch_id&role&status  [user.view]
POST   /api/v1/users                   [user.create]
PATCH  /api/v1/users/:id               [user.edit]
POST   /api/v1/users/:id/roles         [user.assign_role]
POST   /api/v1/users/:id/branches      [user.assign_role]
DELETE /api/v1/users/:id               soft delete  [user.edit]

GET    /api/v1/branches                [branch.view]
POST   /api/v1/branches                [branch.create]
PATCH  /api/v1/branches/:id            [branch.edit]

GET    /api/v1/roles                   [user.view]
GET    /api/v1/permissions             [user.view]
```

## 9. Security requirements

- argon2id, memory 19456 KiB, iterations 2, parallelism 1
- Rate limit login: 5 attempts per phone per 15 min, then exponential lockout. Track by phone **and** by IP.
- Login response identical for unknown user and wrong password — no user enumeration.
- Every auth event writes `audit_log`: login success, login failure, logout, password change, role change, branch assignment.
- Sessions revocable individually or per user by an admin.
- Password minimum 10 characters; check against a common-password list.

## 10. Acceptance tests

- `login_success_returns_tokens`
- `login_wrong_password_and_unknown_user_are_indistinguishable`
- `login_rate_limited_after_five_attempts`
- `expired_access_token_rejected`
- `revoked_session_rejected_even_with_valid_jwt`
- `refresh_rotation_invalidates_old_token`
- `reused_refresh_token_kills_session_family`
- `tenant_id_from_body_is_ignored` — attempt to override, assert JWT value wins
- `cross_tenant_user_fetch_returns_404` — not 403; do not leak existence
- `branch_scoped_user_cannot_act_on_unassigned_branch`
- `only_pharmacist_and_super_admin_have_rx_approve` — iterate all seeded roles
- `permission_denied_returns_403_and_audits`
- `every_auth_event_writes_audit_log`

## 11. Done checklist

- [ ] argon2id hashing with specified parameters
- [ ] JWT access + opaque refresh with rotation and family revocation
- [ ] `TenantContext` extractor; `tenant_id` sourced only from claims
- [ ] Ten system roles seeded with correct permission sets
- [ ] Branch scoping enforced on every branch-scoped endpoint
- [ ] Login rate limiting by phone and IP
- [ ] All auth events audited
- [ ] All 13 acceptance tests green
- [ ] `contracts/openapi.json` regenerated and committed
- [ ] Clippy clean, `cargo sqlx prepare` run
