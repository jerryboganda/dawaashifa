# REVIEW_BRIEF.md — Spec 04 (Identity, RBAC, Branches & Sessions)

## Spec Reference
- **Spec**: `docs/04_IDENTITY_RBAC_BRANCHES.md`
- **Branch**: `feat/04-identity-rbac`

## Invariants Enforced
- **I-1 (Tenant Isolation)**: `tenant_id` is extracted strictly from JWT bearer claims in `TenantContext::from_request_parts` and passed down to queries. Never accepted from request body or query parameter.
- **I-2 (Row-Level Security)**: Enabled across `users`, `sessions`, `roles`, `permissions`, `user_roles`, `user_branches`, `branches`.
- **I-3 (Rx Approval Guard)**: Enforced at permission level. Only `PHARMACIST` and `SUPER_ADMIN` roles hold `rx.approve`. Verified by unit test asserting all other 8 system roles do not possess `rx.approve`.
- **I-7 (Repository & Safe SQL)**: All SQL queries use parameterized typed binds via SQLx.
- **I-9 (Audit Logging)**: Every auth event (login success, login failure, logout, session rotation, password change) writes an immutable `audit_log` row.

## What Was Built
1. **Argon2id Password Hashing**: Hashing and verification with params `m=19456, t=2, p=1` + blacklist validation for common passwords.
2. **JWT & Session Management**:
   - 15-minute HS256 access tokens.
   - 30-day opaque 32-byte refresh tokens with SHA-256 database hashing.
   - Refresh token rotation with automatic session family revocation upon token reuse.
3. **Role-Based Access Control**:
   - 10 seeded system roles: `SUPER_ADMIN`, `OPERATIONS_HEAD`, `BRANCH_MANAGER`, `PHARMACIST`, `PHARMACY_ASSISTANT`, `INVENTORY_CONTROLLER`, `ACCOUNTANT`, `RIDER`, `B2B_DESK`, `AUDITOR`.
   - Branch scoping (`can_act_on_branch` / `require_branch`).
4. **Axum HTTP Layer & OpenAPI**:
   - Auth endpoints: `/api/v1/auth/login`, `/refresh`, `/logout`, `/me`, `/password/change`.
   - Admin endpoints: `/api/v1/users`, `/branches`, `/roles`, `/permissions`.
   - Automated OpenAPI specification emitter emitting `contracts/openapi.json`.
   - `@shifa/shared` type generation via `pnpm gen:api`.

## Acceptance Tests Verification
- `cargo test --workspace` passed 18 tests (9 core, 1 db migration/RLS, 5 identity/roles/crypto, 2 password hashing/verification, 1 API auth & session lifecycle).
- `cargo clippy --workspace --all-targets -- -D warnings` passed with 0 warnings.
- `cargo fmt --all --check` passed.
- `pnpm check && pnpm lint && pnpm test` passed cleanly.
