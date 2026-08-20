# DOC 08 — AI ORCHESTRATION, LANGUAGE PIPELINE & CONFIDENCE GATING

**Agent:** Backend (Copilot)
**Depends on:** Docs 01, 05, 07
**Produces:** `crates/ai`
**Branch:** `feat/08-ai-orchestration`

---

## 1. Objective

The AI gateway and language pipeline. All models are external, served from a separate GPU host over an OpenAI-compatible HTTP API. **The platform never loads a model in-process.**

## 2. In scope

- Provider-agnostic AI client (chat, vision, STT, embeddings)
- Per-task model configuration with timeouts and circuit breakers
- Script detection and Roman Urdu normalisation
- Intent classification and entity extraction
- Voice note transcription
- Reply generation with confidence scoring
- Confidence gating and human escalation
- Prompt versioning with golden-file tests
- Feedback loop consuming override events from Doc 07

## 3. Out of scope — do NOT build

- Prescription OCR workflow (Doc 09 — this doc provides the vision call, Doc 09 owns the workflow)
- Model hosting, serving, or fine-tuning
- Any auto-send in an Rx context (invariant I-6)
- Cart or order mutation

## 4. Gateway contract

The platform speaks **OpenAI-compatible HTTP only**, so models can be swapped or failed over to a hosted provider without code changes.

```
POST {AI_BASE_URL}/v1/chat/completions      chat + vision
POST {AI_BASE_URL}/v1/audio/transcriptions  STT
POST {AI_BASE_URL}/v1/embeddings            embeddings
```

```toml
[ai]
base_url = "${AI_BASE_URL}"
api_key  = "${AI_API_KEY}"

[ai.tasks.intent]    model="qwen3-instruct"   timeout_ms=4000  max_retries=2
[ai.tasks.reply]     model="qwen3-instruct"   timeout_ms=8000  max_retries=1
[ai.tasks.rx_ocr]    model="qwen3-vl"         timeout_ms=25000 max_retries=2
[ai.tasks.stt]       model="whisper-large-v3" timeout_ms=20000 max_retries=2
[ai.tasks.embed]     model="bge-m3"           timeout_ms=3000  max_retries=3

[ai.fallback]            # optional hosted provider for degraded operation
enabled  = false
base_url = ""
```

```rust
#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat(&self, t: Task, req: ChatRequest) -> Result<ChatResponse, AiError>;
    async fn vision(&self, t: Task, req: VisionRequest) -> Result<ChatResponse, AiError>;
    async fn transcribe(&self, t: Task, audio: AudioBytes, hint: Option<Locale>)
        -> Result<Transcript, AiError>;
    async fn embed(&self, t: Task, texts: &[String]) -> Result<Vec<Vec<f32>>, AiError>;
}
```

Circuit breaker per task: opens after 5 consecutive failures, half-opens after 30s. **On open circuit, work is queued for a human — never dropped, never silently degraded.**

Every call logs to `ai_invocations` (task, model, tokens, latency, outcome) for cost tracking.

## 5. Language pipeline

```
inbound text
  → script detect
  → if Latin: Roman-Urdu vs English classifier
  → if Roman Urdu: normalise
  → intent + entity extraction
  → entity resolution via crates/catalog (Doc 05)
```

### 5.1 Script detection
Character-block based, deterministic, no model call. Arabic block → Urdu script. Latin → classifier. Mixed → treat as code-mixed, run both paths, merge.

### 5.2 Roman Urdu normalisation
Rule-based, deterministic, **runs before any model call**:

```
kh→k    ph→f    gh→g    th→t    dh→d    ch→c
ee→i    oo→u    aa→a    ai→e    au→o
trailing silent h dropped
doubled consonants collapsed
Arabic-Indic digits ٠-٩ → 0-9
```

Common forms that must all normalise identically:
`mujhe / mujay / mujhy / muje / mjhe` → `muje`
`chahiye / chahiyay / chaiye / chahye` → `caye`
`kitne / kitnay / kitny` → `kitne`

**Do not rely on the LLM for this.** The normaliser plus the alias table from Doc 05 carry most of the accuracy. The model handles intent, not spelling.

### 5.3 Intents
```
PRODUCT_ENQUIRY   PRICE_ENQUIRY     AVAILABILITY_CHECK
PLACE_ORDER       ORDER_STATUS      CANCEL_ORDER
PRESCRIPTION_UPLOAD                 DELIVERY_ENQUIRY
PAYMENT_QUERY     COMPLAINT         GREETING
HUMAN_REQUEST     OTHER
```

`HUMAN_REQUEST` and `COMPLAINT` always route to a human immediately, regardless of confidence.

## 6. Voice notes

Pakistani customers send voice notes constantly. Pipeline: download → convert to 16kHz mono WAV → STT with locale hint → normalise transcript → intent extraction.

- Languages: Urdu, Punjabi, Pashto, Sindhi, English, code-mixed
- Store the transcript alongside the audio; a human reviewing the conversation must be able to read what was said
- Transcript confidence below 0.70 → escalate to human with the audio attached, do not act on a guess
- Voice notes over 3 minutes: transcribe but always escalate — long messages are usually complex

## 7. Confidence gating

```rust
pub struct AiOutcome<T> {
    pub value: T,
    pub confidence: f32,
    pub escalate: bool,
    pub escalation_reason: Option<String>,
}
```

| Condition | Action |
|---|---|
| Rx-related output, any confidence | **Always** pharmacist queue (I-6) |
| Controlled substance mentioned | Always human |
| Intent = `HUMAN_REQUEST` or `COMPLAINT` | Always human |
| Confidence < 0.60 | Human queue |
| 0.60–0.85 | Draft for staff approval (`PENDING_APPROVAL`) |
| > 0.85, non-Rx | Draft for staff approval — auto-send is a per-tenant setting, **default off** |
| Circuit breaker open | Human queue |

**There is no path where low confidence produces silence.** Acknowledge to the customer, queue the human.

Auto-send, where enabled, is permitted only for: `GREETING`, `ORDER_STATUS`, `DELIVERY_ENQUIRY`. Never for pricing, availability, or anything Rx.

## 8. Prompt management

```
crates/ai/prompts/
  intent_classify.v3.md
  reply_generate.v2.md
  rx_extract.v4.md
  entity_extract.v2.md
```

- Versioned filenames. Never edit a version in place — add a new one.
- `ai_invocations.prompt_version` records which was used
- **Golden-file tests**: each prompt has fixtures in `tests/golden/{prompt}/` with input and expected structured output. A prompt change must show its output diff in the PR.
- All prompts include the Pakistani context block: currency PKR, local brand names, Roman Urdu examples, code-mixing examples.

## 9. Feedback loop

Consume `conversation.reply_overridden` from Doc 07:

```sql
ai_feedback(id, tenant_id, conversation_id, message_id, task, prompt_version,
            ai_output TEXT, human_output TEXT, edit_distance,
            intent, confidence, created_at)
```

Weekly report: override rate by intent, by prompt version, by confidence band. A rising override rate in a band means the threshold needs raising. This is how you tune the gates with evidence rather than guesswork.

Also call `catalog::learn_alias` when an override corrects a product identification.

## 10. Endpoints

```
POST /api/v1/ai/analyse        {conversation_id, message_id} → intent, entities, confidence
POST /api/v1/ai/draft-reply    {conversation_id} → draft (never auto-sent)
POST /api/v1/ai/transcribe     {message_id} → transcript
GET  /api/v1/ai/health         circuit breaker state per task
GET  /api/v1/ai/usage          ?from&to  token and cost report [report.view]
GET  /api/v1/ai/feedback       ?from&to  override analytics [report.view]
```

## 11. Acceptance tests

- `script_detection_table` — Urdu, English, Roman Urdu, code-mixed
- `roman_urdu_normaliser_table` — all documented equivalences, 60+ cases
- `normaliser_runs_before_model_call` — asserts ordering
- `intent_classification_golden_files`
- `human_request_always_escalates_regardless_of_confidence`
- `complaint_always_escalates`
- `rx_output_always_queues_for_pharmacist` — even at confidence 0.99
- `controlled_substance_always_escalates`
- `low_confidence_escalates_and_still_acknowledges_customer`
- `circuit_open_queues_for_human_not_drops`
- `autosend_disabled_by_default`
- `autosend_never_applies_to_pricing_or_rx` — even when enabled
- `voice_note_low_confidence_escalates_with_audio_attached`
- `long_voice_note_always_escalates`
- `override_event_creates_feedback_row_and_learns_alias`
- `ai_invocation_logged_with_tokens_and_version`

All model calls mocked. No test hits the GPU host.

## 12. Done checklist

- [ ] OpenAI-compatible client, per-task config, circuit breakers
- [ ] Optional hosted fallback provider, disabled by default
- [ ] Deterministic script detection and Roman Urdu normaliser running pre-model
- [ ] Intent classification with the 13 documented intents
- [ ] Voice note transcription with locale hints and escalation rules
- [ ] Confidence gating table implemented exactly; auto-send default off
- [ ] Versioned prompts with golden-file tests
- [ ] Feedback loop writing `ai_feedback` and calling `learn_alias`
- [ ] `ai_invocations` cost tracking
- [ ] All 16 acceptance tests green
- [ ] `contracts/openapi.json` regenerated
