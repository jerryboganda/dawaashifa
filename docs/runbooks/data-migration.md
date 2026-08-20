# RUNBOOK — Data Migration Toolkit

## 1. Overview
The Data Migration Toolkit (`migration-tool`) imports product masters, customers, and historical sales from legacy databases, CSV files, and JSON exports with complete reversibility.

## 2. Mandatory Workflow: Always Dry Run First
**No operator should ever run a commit without reading a dry-run report first.**

```bash
# Step 1: Probe the source
migration-tool probe --source csv --file-path data/legacy_products.csv

# Step 2: Validate mapping YAML
migration-tool validate --mapping mappings/legacy_pos_products.yaml

# Step 3: Run Dry Run (writes nothing to live tables)
migration-tool run --mapping mappings/legacy_pos_products.yaml

# Step 4: Inspect report, rejections, and sample transformed records
# Step 5: Execute Commit only after verification
migration-tool run --mapping mappings/legacy_pos_products.yaml --commit
```

## 3. Rollback Procedure
```bash
# Roll back by batch ID
migration-tool rollback --batch-id <BATCH_UUID>
```

> **Safety Rule**: Rollback is automatically refused if any imported product has already been sold in customer orders or moved in stock movements.
