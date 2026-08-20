use shifa_ai::gating::evaluate_gating;
use shifa_ai::language::{detect_script, normalise_roman_urdu};
use shifa_ai::models::*;
use shifa_ai::service::AiService;
use shifa_core::context::TenantContext;
use shifa_core::id::{ConversationId, MessageId, ProductId, TenantId, UserId};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashSet;
use uuid::Uuid;

fn create_test_context(tenant_id: TenantId) -> TenantContext {
    let mut permissions = HashSet::new();
    permissions.insert("report.view".to_string());
    permissions.insert("product.create".to_string());

    TenantContext::from_verified_claims(
        tenant_id,
        UserId::new(),
        vec![],
        permissions,
        vec!["SUPER_ADMIN".to_string()],
        true,
    )
}

#[test]
fn test_script_detection_table() {
    // Urdu text (\u{0645}\u{062C}\u{06BE}\u{06D2} \u{067E}\u{06CC}\u{0646}\u{0627}\u{0688}\u{0648}\u{0644} \u{0686}\u{0627}\u{06C1}\u{06CC}\u{06D2})
    assert_eq!(detect_script("\u{0645}\u{062C}\u{06BE}\u{06D2} \u{067E}\u{06CC}\u{0646}\u{0627}\u{0688}\u{0648}\u{0644} \u{0686}\u{0627}\u{06C1}\u{06CC}\u{06D2}"), CustomerScript::Urdu);
    assert_eq!(
        detect_script("Please send two packs of Panadol Extra"),
        CustomerScript::English
    );
    assert_eq!(
        detect_script("muje panadol extra chahiye do dabbi"),
        CustomerScript::RomanUrdu
    );
    assert_eq!(
        detect_script(
            "mujhe 2 dabbi \u{067E}\u{06CC}\u{0646}\u{0627}\u{0688}\u{0648}\u{0644} chahiye"
        ),
        CustomerScript::CodeMixed
    );
}

#[test]
fn test_roman_urdu_normaliser_table() {
    // 1. mujhe variants -> muje
    for variant in &["mujhe", "mujay", "mujhy", "muje", "mjhe"] {
        assert_eq!(
            normalise_roman_urdu(variant),
            "muje",
            "Failed on {}",
            variant
        );
    }

    // 2. chahiye variants -> caye
    for variant in &["chahiye", "chahiyay", "chaiye", "chahye", "chahiya"] {
        assert_eq!(
            normalise_roman_urdu(variant),
            "caye",
            "Failed on {}",
            variant
        );
    }

    // 3. kitne variants -> kitne
    for variant in &["kitne", "kitnay", "kitny", "kitna"] {
        assert_eq!(
            normalise_roman_urdu(variant),
            "kitne",
            "Failed on {}",
            variant
        );
    }

    // 4. Arabic-Indic digits (\u{0662}=2, \u{0665}\u{0660}\u{0660}=500)
    assert_eq!(
        normalise_roman_urdu("\u{0662} dabbi panadol"),
        "2 dabi panadol"
    );
    assert_eq!(
        normalise_roman_urdu("\u{0665}\u{0660}\u{0660} rupees"),
        "500 rupis"
    );

    // 5. Letter transforms (kh->k, ph->f, gh->g, th->t, dh->d, ch->c, ee->i, oo->u)
    assert_eq!(normalise_roman_urdu("khana"), "kana");
    assert_eq!(normalise_roman_urdu("phir"), "fir");
    assert_eq!(normalise_roman_urdu("ghar"), "gar");
    assert_eq!(normalise_roman_urdu("thoda"), "toda");
    assert_eq!(normalise_roman_urdu("doodh"), "dud");
    assert_eq!(normalise_roman_urdu("theek"), "tik");
}

#[test]
fn test_confidence_gating_rules() {
    // 1. Human request always escalates regardless of confidence (even 0.99)
    let g1 = evaluate_gating(IntentType::HumanRequest, 0.99, false, false, false, false);
    assert!(g1.escalate_to_human);

    // 2. Complaint always escalates
    let g2 = evaluate_gating(IntentType::Complaint, 0.95, false, false, false, false);
    assert!(g2.escalate_to_human);

    // 3. Rx output always queues for pharmacist, even at confidence 0.99 (Invariant I-6)
    let g3 = evaluate_gating(IntentType::ProductEnquiry, 0.99, true, false, false, false);
    assert!(g3.escalate_to_human);
    assert!(g3.requires_pharmacist);
    assert!(!g3.can_auto_send);

    // 4. Controlled substance always escalates
    let g4 = evaluate_gating(IntentType::ProductEnquiry, 0.95, false, true, false, false);
    assert!(g4.escalate_to_human);
    assert!(g4.requires_pharmacist);

    // 5. Low confidence (< 0.60) escalates
    let g5 = evaluate_gating(IntentType::ProductEnquiry, 0.55, false, false, false, false);
    assert!(g5.escalate_to_human);

    // 6. Circuit breaker open always queues for human
    let g6 = evaluate_gating(IntentType::Greeting, 0.95, false, false, true, false);
    assert!(g6.escalate_to_human);

    // 7. Auto-send disabled by default
    let g7 = evaluate_gating(IntentType::Greeting, 0.95, false, false, false, false);
    assert!(!g7.can_auto_send);

    // 8. Auto-send never applies to pricing or Rx even when enabled
    let g8 = evaluate_gating(IntentType::PriceEnquiry, 0.95, false, false, false, true);
    assert!(!g8.can_auto_send);

    let g9 = evaluate_gating(IntentType::Greeting, 0.95, false, false, false, true);
    assert!(g9.can_auto_send);
}

#[tokio::test]
async fn test_ai_pipeline_voice_notes_and_feedback_integration() {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://shifa:shifa_password@localhost:5432/shifa".to_string());
    let pool = match PgPoolOptions::new()
        .max_connections(15)
        .connect(&database_url)
        .await
    {
        Ok(p) => p,
        Err(_) => {
            println!("Skipping DB-backed AI test: local postgres not reachable");
            return;
        }
    };

    let tenant_id = TenantId::new();
    let ctx = create_test_context(tenant_id);

    // Seed tenant and product
    sqlx::query(
        "INSERT INTO tenants (id, name, slug, tier, status)
         VALUES ($1, 'AI Test Tenant', $2, 'STANDARD', 'ACTIVE')",
    )
    .bind(tenant_id.0)
    .bind(format!("ai-test-{}", Uuid::now_v7()))
    .execute(&pool)
    .await
    .unwrap();

    let product_id = ProductId::new();
    sqlx::query(
        "INSERT INTO products (id, tenant_id, brand_name, generic_name, mrp, is_prescription_only, status)
         VALUES ($1, $2, 'Disprin 300mg', 'Aspirin', 50.00, false, 'ACTIVE')"
    )
    .bind(product_id.0)
    .bind(tenant_id.0)
    .execute(&pool)
    .await
    .unwrap();

    let ai_service = AiService::new(pool.clone());
    let conv_id = ConversationId::new();
    let msg_id = MessageId::new();

    // 1. Acceptance test: normaliser runs before model call & logs invocation
    let analysis = ai_service
        .analyse_message(
            &ctx,
            AiAnalyseRequest {
                conversation_id: conv_id,
                message_id: msg_id,
                raw_text: "mujhe panadol extra chahiye".into(),
                is_rx_context: false,
                contains_controlled_substance: false,
            },
        )
        .await
        .unwrap();

    assert_eq!(analysis.normalised_text, "muje panadol extra caye");
    assert_eq!(analysis.intent, IntentType::ProductEnquiry);

    let inv_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_invocations WHERE tenant_id = $1 AND conversation_id = $2",
    )
    .bind(tenant_id.0)
    .bind(conv_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        inv_count >= 1,
        "AI invocation must be logged to ai_invocations table"
    );

    // 2. Acceptance test: voice note transcription & escalation rules
    let stt_short = ai_service
        .transcribe_voice_note(
            &ctx,
            AiTranscribeRequest {
                message_id: msg_id,
                audio_url: "https://audio.example.com/note1.wav".into(),
                duration_seconds: 45,
                locale_hint: Some("ur-PK".into()),
            },
        )
        .await
        .unwrap();
    assert!(!stt_short.escalate);
    assert_eq!(
        stt_short.normalised_transcript,
        "muje panadol extra caye do dabi"
    );

    let stt_long = ai_service
        .transcribe_voice_note(
            &ctx,
            AiTranscribeRequest {
                message_id: msg_id,
                audio_url: "https://audio.example.com/long_note.wav".into(),
                duration_seconds: 240, // 4 mins > 3 mins -> must escalate
                locale_hint: Some("ur-PK".into()),
            },
        )
        .await
        .unwrap();
    assert!(stt_long.escalate);

    // 3. Acceptance test: override event creates feedback row and learns alias
    ai_service
        .record_feedback(
            &ctx,
            FeedbackEventRequest {
                conversation_id: conv_id,
                message_id: msg_id,
                task: "reply".into(),
                prompt_version: "reply_generate.v2".into(),
                ai_output: "Panadol available hai".into(),
                human_output: "Disprin available hai".into(),
                intent: "PRODUCT_ENQUIRY".into(),
                confidence: 0.88,
                corrected_alias: Some(("desprin".into(), "Disprin 300mg".into())),
            },
        )
        .await
        .unwrap();

    let feedback_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_feedback WHERE tenant_id = $1 AND conversation_id = $2",
    )
    .bind(tenant_id.0)
    .bind(conv_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(feedback_count, 1);

    let alias_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM product_aliases WHERE tenant_id = $1 AND alias = 'desprin'",
    )
    .bind(tenant_id.0)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        alias_count, 1,
        "Corrected alias must be dynamically learned in catalog"
    );
}
