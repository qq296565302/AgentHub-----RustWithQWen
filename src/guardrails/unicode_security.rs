use unicode_normalization::UnicodeNormalization;

#[allow(dead_code)]
pub struct UnicodeSecurityGuard;

#[allow(dead_code)]
impl UnicodeSecurityGuard {
    pub fn new() -> Self {
        Self
    }

    pub fn normalize(&self, input: &str) -> String {
        input.nfc().collect()
    }

    pub fn has_homoglyphs(&self, input: &str) -> bool {
        let suspicious_chars = vec![
            '\u{0430}',
            '\u{0435}',
            '\u{043E}',
            '\u{0440}',
            '\u{0441}',
            '\u{0443}',
            '\u{0445}',
        ];
        input.chars().any(|c| suspicious_chars.contains(&c))
    }

    pub fn has_zero_width_chars(&self, input: &str) -> bool {
        let zero_width_chars = vec![
            '\u{200B}',
            '\u{200C}',
            '\u{200D}',
            '\u{FEFF}',
            '\u{2060}',
        ];
        input.chars().any(|c| zero_width_chars.contains(&c))
    }
}
