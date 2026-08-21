# Runbook: FBR Invoicing Outage & Catch-up

## Symptoms
- Alert `FbrQueueDepthHigh` firing (`fbr_queue_depth > 100` or items pending > 6 hours).
- FBR POS API returning HTTP 5xx, connection timeouts, or SSL handshake errors.

## Immediate Action
1. Verify customer order intake is unaffected: FBR outages **never block order confirmation** (Invariant / Doc 13 §9).
2. Check provisional invoice emission: After 30 minutes in `PENDING` queue, orders automatically emit provisional PDF receipts with disclaimer.

## Diagnosis
```bash
# Check queue status via API
curl -s -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/v1/fbr/queue-status | jq .

# Inspect worker logs for FBR error messages
docker logs shifa-worker | grep -i "fbr"
```

## Resolution & Catch-up
1. Once FBR API connectivity is restored, the `shifa-worker` FBR queue consumer will automatically resume submissions with exponential backoff and jitter.
2. If manual trigger is required for stuck items:
```bash
# Force retry of pending/stale invoices
curl -X POST -H "Authorization: Bearer $TOKEN" http://localhost:8080/api/v1/invoices/retry-queue
```
3. Verify that accepted invoices receive official `fiscal_invoice_no` and FBR QR payload.

## Prevention
- Maintain the local gapless numbering sequence independent of FBR reference.
- Keep retry backoff capped at 6 hours with max 10 attempts before flagging for manual ops review.
