use std::collections::HashMap;

/// All standardized permission keys across the platform per Doc 04 §5.
pub const ALL_PERMISSIONS: &[&str] = &[
    "rx.view",
    "rx.approve",
    "rx.reject",
    "order.view",
    "order.create",
    "order.edit",
    "order.cancel",
    "order.refund",
    "payment.view",
    "payment.approve",
    "payment.reject",
    "payment.refund",
    "inventory.view",
    "inventory.receive",
    "inventory.adjust",
    "inventory.transfer",
    "inbox.view",
    "inbox.reply",
    "inbox.override",
    "inbox.assign",
    "product.view",
    "product.create",
    "product.edit",
    "product.price",
    "branch.view",
    "branch.create",
    "branch.edit",
    "user.view",
    "user.create",
    "user.edit",
    "user.assign_role",
    "report.view",
    "report.export",
    "audit.view",
    "tenant.settings",
    "b2b.quote",
    "b2b.credit",
];

/// The 10 seeded system roles per Doc 04 §4.
pub fn get_system_role_definitions() -> HashMap<&'static str, Vec<&'static str>> {
    let mut m = HashMap::new();

    // 1. SUPER_ADMIN: everything, all branches, tenant settings
    m.insert("SUPER_ADMIN", ALL_PERMISSIONS.to_vec());

    // 2. OPERATIONS_HEAD: all branches, all operational permissions, no tenant settings, NO rx.approve (Invariant I-3)
    let ops_perms = ALL_PERMISSIONS
        .iter()
        .copied()
        .filter(|&p| p != "tenant.settings" && p != "rx.approve")
        .collect::<Vec<_>>();
    m.insert("OPERATIONS_HEAD", ops_perms);

    // 3. BRANCH_MANAGER: assigned branches: inbox, orders, inventory, payment approval, reply override
    m.insert(
        "BRANCH_MANAGER",
        vec![
            "inbox.view",
            "inbox.reply",
            "inbox.override",
            "inbox.assign",
            "order.view",
            "order.create",
            "order.edit",
            "order.cancel",
            "inventory.view",
            "inventory.receive",
            "inventory.adjust",
            "inventory.transfer",
            "payment.view",
            "payment.approve",
            "payment.reject",
            "product.view",
            "branch.view",
            "user.view",
            "report.view",
        ],
    );

    // 4. PHARMACIST: rx.approve, inbox read, product read, order read
    // Invariant I-3: rx.approve belongs only to PHARMACIST and SUPER_ADMIN
    m.insert(
        "PHARMACIST",
        vec![
            "rx.view",
            "rx.approve",
            "rx.reject",
            "inbox.view",
            "inbox.reply",
            "product.view",
            "order.view",
        ],
    );

    // 5. PHARMACY_ASSISTANT: inbox, orders, inventory read — no approvals
    m.insert(
        "PHARMACY_ASSISTANT",
        vec![
            "inbox.view",
            "inbox.reply",
            "order.view",
            "order.create",
            "inventory.view",
            "product.view",
        ],
    );

    // 6. INVENTORY_CONTROLLER: stock receipt, adjustment, transfer, batch management
    m.insert(
        "INVENTORY_CONTROLLER",
        vec![
            "inventory.view",
            "inventory.receive",
            "inventory.adjust",
            "inventory.transfer",
            "product.view",
            "product.create",
            "product.edit",
            "product.price",
        ],
    );

    // 7. ACCOUNTANT: payments, invoices, reconciliation, reports — read-only on orders
    m.insert(
        "ACCOUNTANT",
        vec![
            "payment.view",
            "payment.approve",
            "payment.reject",
            "payment.refund",
            "order.view",
            "report.view",
            "report.export",
        ],
    );

    // 8. RIDER: own deliveries only, via scoped token
    m.insert("RIDER", vec!["order.view"]);

    // 9. B2B_DESK: quotes, hospital accounts, credit limits
    m.insert(
        "B2B_DESK",
        vec![
            "b2b.quote",
            "b2b.credit",
            "order.view",
            "order.create",
            "product.view",
            "payment.view",
        ],
    );

    // 10. AUDITOR: read-only across everything, including audit log
    m.insert(
        "AUDITOR",
        vec![
            "audit.view",
            "report.view",
            "report.export",
            "rx.view",
            "order.view",
            "payment.view",
            "inventory.view",
            "product.view",
            "branch.view",
            "user.view",
        ],
    );

    m
}
