use crate::models::CustomerScript;

/// Detect customer script based on unicode character blocks per Doc 08 §5.1.
pub fn detect_script(text: &str) -> CustomerScript {
    let mut has_arabic = false;
    let mut has_latin = false;

    for c in text.chars() {
        if ('\u{0600}'..='\u{06FF}').contains(&c)
            || ('\u{0750}'..='\u{077F}').contains(&c)
            || ('\u{FB50}'..='\u{FDFF}').contains(&c)
            || ('\u{FE70}'..='\u{FEFF}').contains(&c)
        {
            has_arabic = true;
        } else if c.is_ascii_alphabetic() {
            has_latin = true;
        }
    }

    if has_arabic && has_latin {
        CustomerScript::CodeMixed
    } else if has_arabic {
        CustomerScript::Urdu
    } else if has_latin {
        let lower = text.to_lowercase();
        if lower.contains("chahiye")
            || lower.contains("chaiye")
            || lower.contains("muje")
            || lower.contains("mujhe")
            || lower.contains("bhejo")
            || lower.contains("kitne")
            || lower.contains("hai")
            || lower.contains("karna")
            || lower.contains("dabbi")
            || lower.contains("rupees")
            || lower.contains("bhai")
        {
            CustomerScript::RomanUrdu
        } else {
            CustomerScript::English
        }
    } else {
        CustomerScript::English
    }
}

/// Rule-based deterministic Roman Urdu normaliser per Doc 08 §5.2.
/// Runs BEFORE any model call.
pub fn normalise_roman_urdu(input: &str) -> String {
    let mut s = input.to_lowercase();

    // 1. Convert Arabic-Indic digits \u{0660}-\u{0669} and Eastern Arabic \u{06F0}-\u{06F9} to 0-9
    let standard_arabic_digits = [
        '\u{0660}', '\u{0661}', '\u{0662}', '\u{0663}', '\u{0664}', '\u{0665}', '\u{0666}',
        '\u{0667}', '\u{0668}', '\u{0669}',
    ];
    let eastern_arabic_digits = [
        '\u{06F0}', '\u{06F1}', '\u{06F2}', '\u{06F3}', '\u{06F4}', '\u{06F5}', '\u{06F6}',
        '\u{06F7}', '\u{06F8}', '\u{06F9}',
    ];

    for (i, &ad) in standard_arabic_digits.iter().enumerate() {
        s = s.replace(ad, &i.to_string());
    }
    for (i, &ead) in eastern_arabic_digits.iter().enumerate() {
        s = s.replace(ead, &i.to_string());
    }

    // Direct mapping of common variants to canonical forms
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut norm_words = Vec::new();

    for w in words {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
        let norm: String = match clean {
            "mujhe" | "mujay" | "mujhy" | "muje" | "mjhe" => "muje".into(),
            "chahiye" | "chahiyay" | "chaiye" | "chahye" | "chahiya" => "caye".into(),
            "kitne" | "kitnay" | "kitny" | "kitna" => "kitne".into(),
            "panadol" => "panadol".into(),
            "bhejo" | "bhejain" | "bhejdein" => "bejo".into(),
            "shukriya" | "shukria" => "sukriya".into(),
            other => {
                let mut trans = other.to_string();
                // Substring transforms
                trans = trans.replace("kh", "k");
                trans = trans.replace("ph", "f");
                trans = trans.replace("gh", "g");
                trans = trans.replace("th", "t");
                trans = trans.replace("dh", "d");
                trans = trans.replace("ch", "c");
                trans = trans.replace("ee", "i");
                trans = trans.replace("oo", "u");
                trans = trans.replace("aa", "a");
                trans = trans.replace("ai", "e");
                trans = trans.replace("au", "o");

                // Drop trailing silent 'h' if length > 3
                if trans.len() > 3 && trans.ends_with('h') {
                    trans.pop();
                }

                // Collapse doubled consonants
                let mut collapsed = String::new();
                let mut prev: Option<char> = None;
                for c in trans.chars() {
                    if let Some(p) = prev {
                        if p == c && c.is_alphabetic() && !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
                        {
                            continue;
                        }
                    }
                    collapsed.push(c);
                    prev = Some(c);
                }
                collapsed
            }
        };
        norm_words.push(norm);
    }

    norm_words.join(" ")
}
