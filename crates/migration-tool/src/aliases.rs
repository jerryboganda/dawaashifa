use std::collections::HashSet;

pub struct AliasGenerator;

impl AliasGenerator {
    /// Generates search aliases and misspellings for an imported product (Doc 15 §10)
    pub fn generate_aliases(names: &[String], generic_name: Option<&str>) -> Vec<String> {
        let mut set = HashSet::new();

        for name in names {
            let clean = name.trim();
            if clean.is_empty() {
                continue;
            }

            // 1. Original name
            set.insert(clean.to_string());
            set.insert(clean.to_lowercase());

            // 2. Transposed characters (swap adjacent letters)
            let chars: Vec<char> = clean.chars().collect();
            if chars.len() > 3 {
                for i in 1..chars.len() - 1 {
                    let mut transposed = chars.clone();
                    transposed.swap(i, i + 1);
                    set.insert(transposed.into_iter().collect());
                }
            }

            // 3. Doubled letters
            if chars.len() > 3 {
                for i in 1..chars.len() {
                    let mut doubled = chars.clone();
                    doubled.insert(i, chars[i]);
                    set.insert(doubled.into_iter().collect());
                }
            }

            // 4. Dropped vowels
            let no_vowels: String = clean
                .chars()
                .filter(|c| !matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u'))
                .collect();
            if no_vowels.len() >= 3 {
                set.insert(no_vowels);
            }
        }

        // 5. Generic name alias
        if let Some(gen) = generic_name {
            if !gen.trim().is_empty() {
                set.insert(gen.trim().to_string());
                set.insert(gen.trim().to_lowercase());
            }
        }

        set.into_iter().collect()
    }
}
