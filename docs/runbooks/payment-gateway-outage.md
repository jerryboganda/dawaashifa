# Runbook: Payment Gateway Outage & Failover

## Symptoms
- Gateway webhook failures (JazzCash, Easypaisa, PayFast, Safepay, Bank Alfalah).
- Customers reporting payment link errors or gateway timeout pages.

## Immediate Action
1. Check gateway status via admin endpoint.
2. If single gateway is down, the system dynamically falls back to secondary payment methods (COD, Bank Transfer slip, or secondary digital gateway).

## Failover Configuration
```bash
# Disable degrading gateway in tenant config
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/settings/gateways/jazzcash \
  -d '{"enabled": false, "failover_to": "easypaisa"}'
```

## Screenshot Proof Fallback (Manual Verification)
- Customers sending direct bank transfer screenshots are routed to the **Payment Proof Review Queue** (`/payments/review`).
- All screenshot payments require explicit human approval with fraud checks (Invariant I-4). Zero auto-approval.

## Recovery
- Re-enable the primary gateway once webhook acknowledgements and health probes return HTTP 200.
