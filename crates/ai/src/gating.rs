use crate::models::IntentType;

#[derive(Debug, Clone)]
pub struct GatingEvaluation {
    pub escalate_to_human: bool,
    pub requires_pharmacist: bool,
    pub can_auto_send: bool,
    pub reason: Option<String>,
}

/// Evaluate confidence gating rules strictly adhering to Doc 08 §7.
pub fn evaluate_gating(
    intent: IntentType,
    confidence: f32,
    is_rx_context: bool,
    contains_controlled_substance: bool,
    circuit_open: bool,
    tenant_allow_auto_send: bool,
) -> GatingEvaluation {
    // 1. Circuit breaker open -> Always human queue
    if circuit_open {
        return GatingEvaluation {
            escalate_to_human: true,
            requires_pharmacist: is_rx_context,
            can_auto_send: false,
            reason: Some("AI Circuit breaker open: escalated to human staff".into()),
        };
    }

    // 2. Controlled substance mentioned -> Always human
    if contains_controlled_substance {
        return GatingEvaluation {
            escalate_to_human: true,
            requires_pharmacist: true,
            can_auto_send: false,
            reason: Some("Controlled substance inquiry requires direct human pharmacist".into()),
        };
    }

    // 3. Rx context, any confidence -> ALWAYS pharmacist queue (Invariant I-6)
    if is_rx_context {
        return GatingEvaluation {
            escalate_to_human: true,
            requires_pharmacist: true,
            can_auto_send: false,
            reason: Some(
                "Rx prescription items require licensed pharmacist approval (Invariant I-6)".into(),
            ),
        };
    }

    // 4. Intent = HUMAN_REQUEST or COMPLAINT -> Always human immediately
    if matches!(intent, IntentType::HumanRequest | IntentType::Complaint) {
        return GatingEvaluation {
            escalate_to_human: true,
            requires_pharmacist: false,
            can_auto_send: false,
            reason: Some(format!(
                "Intent {:?} demands immediate human agent routing",
                intent
            )),
        };
    }

    // 5. Confidence < 0.60 -> Human queue
    if confidence < 0.60 {
        return GatingEvaluation {
            escalate_to_human: true,
            requires_pharmacist: false,
            can_auto_send: false,
            reason: Some(format!(
                "Low AI confidence ({:.2} < 0.60): escalated to staff",
                confidence
            )),
        };
    }

    // 6. Confidence 0.60 - 0.85 -> Draft for staff approval
    if confidence <= 0.85 {
        return GatingEvaluation {
            escalate_to_human: false,
            requires_pharmacist: false,
            can_auto_send: false,
            reason: None,
        };
    }

    // 7. Confidence > 0.85 -> Check tenant auto-send setting and permitted intent categories
    // Auto-send permitted ONLY for: GREETING, ORDER_STATUS, DELIVERY_ENQUIRY.
    // Never for pricing, availability, or Rx!
    let intent_permits_auto_send = matches!(
        intent,
        IntentType::Greeting | IntentType::OrderStatus | IntentType::DeliveryEnquiry
    );

    let can_auto_send = tenant_allow_auto_send && intent_permits_auto_send && !is_rx_context;

    GatingEvaluation {
        escalate_to_human: false,
        requires_pharmacist: false,
        can_auto_send,
        reason: None,
    }
}
