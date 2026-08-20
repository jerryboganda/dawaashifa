//! Flexible reply parsing for unofficial text-based choice and confirm interactions (Doc 03 §6).

pub struct ReplyParser;

impl ReplyParser {
    /// Parses a user's text reply to a Choice prompt into a 1-based option index (1, 2, 3...)
    pub fn parse_choice_index(input: &str) -> Option<usize> {
        let clean = input.trim().to_lowercase();
        if clean.is_empty() {
            return None;
        }

        // Direct ASCII digit
        if let Ok(n) = clean.parse::<usize>() {
            if n > 0 {
                return Some(n);
            }
        }

        // Arabic-Indic / Urdu numerals
        match clean.as_str() {
            "۱" | "١" => return Some(1),
            "۲" | "٢" => return Some(2),
            "۳" | "٣" => return Some(3),
            "۴" | "٤" => return Some(4),
            "۵" | "٥" => return Some(5),
            "۶" | "٦" => return Some(6),
            "۷" | "٧" => return Some(7),
            "۸" | "٨" => return Some(8),
            "۹" | "٩" => return Some(9),
            _ => {}
        }

        // Roman Urdu / Urdu text variants
        if clean.starts_with("option ") || clean.starts_with("opt ") {
            let rest = clean
                .trim_start_matches("option ")
                .trim_start_matches("opt ")
                .trim();
            if let Ok(n) = rest.parse::<usize>() {
                if n > 0 {
                    return Some(n);
                }
            }
        }

        match clean.as_str() {
            "pehla" | "pehli" | "first" | "one" | "aik" | "ایک" | "پہلا" | "پہلی" => {
                Some(1)
            }
            "dusra" | "doosra" | "dusri" | "second" | "two" | "do" | "دو" | "دوسرا" | "دوسری" => {
                Some(2)
            }
            "teesra" | "teesri" | "third" | "three" | "teen" | "تین" | "تیسرا" | "تیسری" => {
                Some(3)
            }
            "chotha" | "chautha" | "chothi" | "fourth" | "four" | "chaar" | "چار" | "چوتھا" => {
                Some(4)
            }
            "panchwa" | "panchwi" | "fifth" | "five" | "paanch" | "پانچ" | "پانچواں" => {
                Some(5)
            }
            _ => None,
        }
    }

    /// Parses a user's text reply to a Confirm prompt into a boolean (true = Yes, false = No)
    pub fn parse_confirm(input: &str) -> Option<bool> {
        let clean = input.trim().to_lowercase();
        if clean.is_empty() {
            return None;
        }

        match clean.as_str() {
            // Affirmatives: English, Urdu, Roman Urdu
            "yes" | "y" | "haan" | "ha" | "ji" | "ji haan" | "ji ha" | "sahi" | "theek" | "ok"
            | "okay" | "confirm" | "zaroor" | "ہاں" | "جی ہاں" | "جی" | "درست" | "صحیح"
            | "ٹھیک" => Some(true),

            // Negatives: English, Urdu, Roman Urdu
            "no" | "n" | "nahi" | "nahin" | "na" | "cancel" | "radd" | "mat karo" | "rok do"
            | "نہں" | "نہیں" | "منسوخ" | "رد" | "نہ" => Some(false),

            _ => None,
        }
    }
}
