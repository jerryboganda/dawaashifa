# Runbook: AI Host Down / Circuit Breaker Open

## Symptoms
- Alert `AiCircuitBreakerOpen` firing (`ai_circuit_breaker_state == OPEN`).
- VLM prescription OCR extraction failing or LLM draft replies failing.
- High AI latency or HTTP 503 from external model endpoints.

## Immediate Action
1. The AI Circuit Breaker automatically falls back to **deterministic triage**:
   - Prescription images land in the pharmacist queue with `raw_ocr_text: "[OCR Unavailable — Please Transcribe Manually]"` and `confidence: 0.0`.
   - WhatsApp customer messages route directly to human agents in the Unified Inbox without AI draft generation.
2. Platform order flow and customer intake remain **100% operational** (Invariant I-6).

## Diagnosis
```bash
# Check AI health endpoint
curl -s http://localhost:8080/api/v1/ai/health | jq .

# Check GPU / model provider status
curl -s http://ai-gpu-host:8000/health
```

## Resolution
1. If self-hosted GPU instance crashed, restart vLLM/Ollama service on the GPU host.
2. If cloud provider rate-limited, failover to secondary provider key via configuration update:
```bash
# Update AI provider config in runtime
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/settings/ai \
  -d '{"primary_provider": "gemini", "fallback_provider": "claude"}'
```
3. Once upstream health check succeeds for 5 consecutive probes, circuit breaker transitions from `HALF_OPEN` to `CLOSED`.

## Prevention
- Ensure multi-provider fallback (Local GPU -> Cloud API -> Human queue).
- Keep deterministic fallback active at all times.
