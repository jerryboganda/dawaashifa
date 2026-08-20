use crate::error::ConversationError;
use std::collections::HashMap;

/// Resolve canned reply template variables strictly per Doc 07 §9.
/// If any {{var}} remains unresolved, returns Err(UnresolvedVariables) to prevent malformed messages reaching customers.
pub fn render_canned_reply(
    template: &str,
    variables: &HashMap<String, String>,
) -> Result<String, ConversationError> {
    let mut rendered = template.to_string();

    for (key, val) in variables {
        let placeholder = format!("{{{{{}}}}}", key);
        rendered = rendered.replace(&placeholder, val);
    }

    // Check if any {{...}} placeholder remains unresolved
    if let Some(start) = rendered.find("{{") {
        if let Some(end) = rendered[start..].find("}}") {
            let unresolved = rendered[start..=start + end + 1].to_string();
            return Err(ConversationError::UnresolvedVariables(unresolved));
        }
    }

    Ok(rendered)
}
