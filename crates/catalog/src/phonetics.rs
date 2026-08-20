/// Normalizes query text by:
/// 1. Converting to lowercase
/// 2. Collapsing whitespace
/// 3. Normalizing Arabic-Indic digits (0-9) to ASCII
/// 4. Stripping Urdu/Arabic diacritics
/// 5. Unifying Urdu letter variations
pub fn normalize_query(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());

    for ch in text.chars() {
        match ch {
            '\u{064B}'..='\u{065F}' | '\u{0670}' => continue,
            '٠' | '۰' => normalized.push('0'),
            '١' | '۱' => normalized.push('1'),
            '٢' | '۲' => normalized.push('2'),
            '٣' | '۳' => normalized.push('3'),
            '٤' | '۴' => normalized.push('4'),
            '٥' | '۵' => normalized.push('5'),
            '٦' | '۶' => normalized.push('6'),
            '٧' | '۷' => normalized.push('7'),
            '٨' | '۸' => normalized.push('8'),
            '٩' | '۹' => normalized.push('9'),
            'ي' | 'ى' | 'ئ' => normalized.push('ی'),
            'ك' => normalized.push('ک'),
            'ة' | 'ھ' => normalized.push('ہ'),
            c if c.is_alphanumeric() || c == ' ' => {
                for lower in c.to_lowercase() {
                    normalized.push(lower);
                }
            }
            _ => normalized.push(' '),
        }
    }

    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Urdu-tuned phonetic encoder per Doc 05 §6.1.
pub fn encode_urdu_phonetic(text: &str) -> String {
    let norm = normalize_query(text);
    if norm.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let chars: Vec<char> = norm.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        let next = if i + 1 < len {
            Some(chars[i + 1])
        } else {
            None
        };

        match (ch, next) {
            ('k', Some('h')) | ('x', _) => {
                result.push('k');
                i += if ch == 'k' { 2 } else { 1 };
                continue;
            }
            ('p', Some('h')) => {
                result.push('f');
                i += 2;
                continue;
            }
            ('g', Some('h')) => {
                result.push('g');
                i += 2;
                continue;
            }
            ('e', Some('e')) => {
                result.push('i');
                i += 2;
                continue;
            }
            ('o', Some('o')) => {
                result.push('u');
                i += 2;
                continue;
            }
            ('a', Some('a')) => {
                result.push('a');
                i += 2;
                continue;
            }
            ('t', Some('h')) => {
                result.push('t');
                i += 2;
                continue;
            }
            ('d', Some('h')) => {
                result.push('d');
                i += 2;
                continue;
            }
            ('c', Some('h')) => {
                result.push('c');
                i += 2;
                continue;
            }
            _ => (),
        }

        match ch {
            'y' => result.push('i'),
            'w' => result.push('u'),
            'z' => result.push('j'),
            'c' | 'q' => result.push('k'),
            'h' if i == len - 1 => (),
            c => result.push(c),
        }

        i += 1;
    }

    let mut collapsed = String::new();
    let mut prev: Option<char> = None;
    for c in result.chars() {
        if Some(c) != prev || c == ' ' {
            collapsed.push(c);
            prev = Some(c);
        }
    }

    let mut root = String::new();
    let chars: Vec<char> = collapsed.chars().collect();
    for (idx, &c) in chars.iter().enumerate() {
        if idx == 0 || !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u') {
            root.push(c);
        }
    }

    root
}
