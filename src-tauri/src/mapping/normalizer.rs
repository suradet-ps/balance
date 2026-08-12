//! Pure-Rust drug-name normalizer and similarity scorer.
//!
//! This module has **no I/O** and no dependency on either source database:
//! the exact same code scores HOSxP names against INVS names everywhere.
//! The rules are deliberately conservative and fully unit-tested against a
//! fixture of real drug names (see `tests` below and `docs/mapping.md`).
//!
//! Normalization order:
//! 1. lowercase;
//! 2. drop parenthetical *dose* content — `(500 mg)` goes away, while
//!    `(พาราเซตามอล)` is kept because it is a translation, not a strength;
//! 3. unify common Thai spelling variants: `รร` → `ร`, `รา` → `ร` (when
//!    followed by a consonant), `รึ` → `ริ`;
//! 4. split into tokens on non-alphanumerics, digit boundaries and script
//!    boundaries (Latin ↔ Thai), so `amoxicillin500mg` and
//!    `Paracetamol(พาราเซตามอล)` both tokenize cleanly;
//! 5. drop pure numbers and dose/unit/dosage-form tokens (`mg`, `มก.`,
//!    `tablet`, `เม็ด`, …);
//! 6. sort + dedupe the surviving tokens — the output is a canonical token
//!    set, so equal normalized strings always mean the same drug name.
//!
//! Scoring cascade (as the roadmap prescribes): normalized equality → token
//! overlap (Jaccard) → Levenshtein similarity on the normalized strings.

/// Minimum score for an un-reviewed `auto` match.  Below this a candidate is
/// shown to the pharmacist but never applied automatically.
pub const AUTO_MATCH_THRESHOLD: f64 = 0.95;

/// Dose / unit / dosage-form tokens that are dropped during normalization.
/// English forms are stripped by token match; Thai forms too (a trailing
/// `มก.` becomes the token `มก` once the period is removed).
const DOSE_TOKENS: &[&str] = &[
    // strengths & units — English
    "mg",
    "mcg",
    "ug",
    "µg",
    "g",
    "ml",
    "cc",
    "iu",
    "unit",
    "units",
    "tab",
    "tabs",
    "tabcap",
    "cap",
    "caps",
    // dosage forms — English
    "tablet",
    "tablets",
    "capsule",
    "capsules",
    "suspension",
    "solution",
    "injection",
    "injections",
    "syrup",
    "cream",
    "gel",
    "ointment",
    "drops",
    "drop",
    "lotion",
    "powder",
    "suppository",
    "susp",
    "sol",
    "inj",
    "syr",
    // strengths & units — Thai (with and without the trailing period)
    "มิลลิกรัม",
    "ไมโครกรัม",
    "มิลลิลิตร",
    "กรัม",
    "ลิตร",
    "ยูนิต",
    "หน่วย",
    "เปอร์เซ็นต์",
    "มก",
    "มล",
    // dosage forms — Thai
    "เม็ด",
    "แคปซูล",
    "ยาเม็ด",
    "ยาน้ำ",
    "ยาฉีด",
    "น้ำเชื่อม",
    "ครีม",
    "เจล",
    "ขี้ผึ้ง",
    "แผ่น",
    "หลอด",
    "แท็บเล็ต",
    "หยด",
];

/// Unicode block of Thai vowels/signs (`\u0E31` – `\u0E4F`): combining
/// marks that `char::is_alphanumeric` does not classify as letters but that
/// are part of Thai spelling and must be preserved.
fn is_thai_mark(c: char) -> bool {
    ('\u{0E30}'..='\u{0E4F}').contains(&c)
}

/// Unicode block of Thai consonants/vowels (`ก` – `ฮ`, `เ` – `ไ`).
fn is_thai(c: char) -> bool {
    ('\u{0E01}'..='\u{0E7F}').contains(&c)
}

/// Normalize a drug name into a canonical sorted token set.
#[must_use]
pub fn normalize(name: &str) -> String {
    let lowered = name.to_lowercase();
    let no_parens = strip_dose_parens(&lowered);
    let unified = unify_thai(&no_parens);
    let tokens = tokenize(&unified);
    let mut kept: Vec<String> = tokens.into_iter().filter(|t| keep_token(t)).collect();
    kept.sort_unstable();
    kept.dedup();
    kept.join(" ")
}

/// Drop parenthesized content that looks like a dose/strength (`(500 mg)`,
/// `(10 มิลลิกรัม)`), but keep content that is a real word (a translation
/// such as `(พาราเซตามอล)` must survive).
fn strip_dose_parens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    let mut buf = String::new();
    for c in s.chars() {
        match c {
            '(' => {
                depth += 1;
                buf.clear();
            }
            ')' if depth > 0 => {
                depth -= 1;
                if !parens_is_dose(&buf) {
                    out.push(' ');
                    out.push_str(&buf);
                }
                buf.clear();
            }
            _ if depth > 0 => buf.push(c),
            _ => out.push(c),
        }
    }
    if depth > 0 {
        // Unbalanced '(' — treat the tail as ordinary text.
        out.push_str(&buf);
    }
    out
}

fn parens_is_dose(content: &str) -> bool {
    if content.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    for tok in tokenize(content) {
        if keep_token(&tok) {
            return false;
        }
    }
    // No surviving token → the parenthetical was only dose/unit words.
    !content.trim().is_empty()
}

/// Unify Thai spelling variants: `รร` → `ร`, `รา` → `ร` (before a
/// consonant), `รึ` → `ริ`.  Applied to the raw spelling before filtering.
fn unify_thai(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // "รร"
        if c == 'ร' && chars.get(i + 1) == Some(&'ร') {
            out.push('ร');
            i += 2;
            continue;
        }
        // "รึ" → "ริ"
        if c == 'ร' && chars.get(i + 1) == Some(&'ึ') {
            out.push('ร');
            out.push('ิ');
            i += 2;
            continue;
        }
        // "รา" → "ร" — the spelling reduction in ธารา/ธาร, การันต์/กรนต์.
        // Applied when the next character is a consonant or end-of-word, but
        // NOT when it is a vowel sign (so "พาราเซตามอล" keeps its "รา").
        if c == 'ร'
            && chars.get(i + 1) == Some(&'า')
            && chars.get(i + 2).is_none_or(|n| !is_thai_mark(*n))
        {
            out.push('ร');
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Split the filtered text into tokens on non-alphanumerics, digit
/// boundaries and script boundaries (Latin ↔ Thai).
fn tokenize(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<(bool, bool)> = None; // (is_digit, is_thai)
    for c in s.chars() {
        let alnum = c.is_alphanumeric() || is_thai_mark(c);
        if !alnum {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            prev = None;
            continue;
        }
        let cls = (c.is_ascii_digit(), is_thai(c));
        if let Some(p) = prev {
            let boundary = cls.0 != p.0 || (cls.0 == p.0 && cls.1 != p.1);
            if boundary && !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c);
        prev = Some(cls);
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

fn keep_token(tok: &str) -> bool {
    if tok.is_empty() {
        return false;
    }
    if tok.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if DOSE_TOKENS.contains(&tok) {
        return false;
    }
    true
}

/// Jaccard overlap of two token sets (as normalized output strings).
fn jaccard(a: &str, b: &str) -> f64 {
    let ta: Vec<&str> = a.split(' ').collect();
    let tb: Vec<&str> = b.split(' ').collect();
    let mut inter = 0usize;
    for t in &ta {
        if tb.contains(t) {
            inter += 1;
        }
    }
    let union = ta.len() + tb.len() - inter;
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Similarity in `[0.0, 1.0]` between two raw drug names.
///
/// Normalized equality → `1.0`; token overlap → `0.5 + 0.5·Jaccard`; otherwise
/// Levenshtein similarity on the normalized strings.  `0.0` when either side
/// normalizes to nothing (nothing comparable — never fabricates a match).
#[must_use]
pub fn similarity(a: &str, b: &str) -> f64 {
    let na = normalize(a);
    let nb = normalize(b);
    if na.is_empty() || nb.is_empty() {
        return 0.0;
    }
    if na == nb {
        return 1.0;
    }
    let jac = jaccard(&na, &nb);
    if jac > 0.0 {
        0.5 + 0.5 * jac
    } else {
        let max_len = na.chars().count().max(nb.chars().count()) as f64;
        if max_len == 0.0 {
            0.0
        } else {
            1.0 - levenshtein(&na, &nb) as f64 / max_len
        }
    }
}

/// Classic two-row Levenshtein edit distance over Unicode scalars.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j + 1] + 1)
                .min(cur[j] + 1)
                .min(prev[j] + usize::from(ca != cb));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Normalization fixtures (real drug names) ─────────────────────

    #[test]
    fn strips_english_strength_with_unit() {
        assert_eq!(normalize("Amoxicillin 500 mg"), "amoxicillin");
        assert_eq!(normalize("Amoxicillin 500mg"), "amoxicillin");
        assert_eq!(normalize("Amoxicillin500mg"), "amoxicillin");
    }

    #[test]
    fn strips_thai_strength_and_dosage_form() {
        assert_eq!(normalize("แอมม็อกซิซิลลิน 500 มิลลิกรัม"), "แอมม็อกซิซิลลิน");
        assert_eq!(normalize("แอมม็อกซิซิลลิน 500 มก. แคปซูล"), "แอมม็อกซิซิลลิน");
        assert_eq!(normalize("พาราเซตามอล 500 มก. (ยาเม็ด)"), "พาราเซตามอล");
        assert_eq!(normalize("โดมเพอริดอน 10 มก."), normalize("โดมเพอริดอน 10mg"));
    }

    #[test]
    fn drops_dose_parens_but_keeps_translations() {
        assert_eq!(
            normalize("Omeprazole 20 mg (โอเมพราโซล)"),
            "omeprazole โอเมพราโซล"
        );
        assert_eq!(
            normalize("Paracetamol (พาราเซตามอล)"),
            "paracetamol พาราเซตามอล"
        );
        assert_eq!(normalize("Amoxicillin (500 mg)"), "amoxicillin");
    }

    #[test]
    fn unifies_thai_spelling_variants() {
        assert_eq!(normalize("บรรจุ"), normalize("บรจุ"));
        assert_eq!(normalize("สรรพคุณ"), normalize("สรพคุณ"));
        assert_eq!(normalize("ธารา"), normalize("ธาร"));
        assert_eq!(normalize("ตรึม"), normalize("ตริม"));
    }

    #[test]
    fn token_set_is_canonical_across_order_and_spacing() {
        assert_eq!(
            normalize("Paracetamol (พาราเซตามอล)"),
            normalize("พาราเซตามอล paracetamol")
        );
        assert_eq!(normalize("vitamin C"), normalize("c vitamin"));
    }

    // ── Similarity fixtures ───────────────────────────────────────────

    #[test]
    fn identical_drugs_score_1() {
        assert_eq!(similarity("Amoxicillin 500 mg", "Amoxicillin 500mg"), 1.0);
        assert_eq!(
            similarity("พาราเซตามอล 500 มก.", "พาราเซตามอล 500 มิลลิกรัม"),
            1.0
        );
        assert_eq!(
            similarity("Paracetamol (พาราเซตามอล)", "พาราเซตามอล paracetamol"),
            1.0
        );
    }

    #[test]
    fn dose_only_difference_still_matches() {
        // Documented limitation: strengths are stripped, so different
        // strengths of the same drug match — the pharmacist reviews.
        assert_eq!(similarity("Aspirin 81 mg", "Aspirin 325 mg"), 1.0);
    }

    #[test]
    fn partial_token_overlap_scores_between() {
        assert_eq!(similarity("Paracetamol + Codeine", "Paracetamol"), 0.75);
        assert_eq!(similarity("Amoxicillin", "Amoxicillin + Clavulanate"), 0.75);
    }

    #[test]
    fn unrelated_drugs_score_low() {
        let s = similarity("Ciprofloxacin 500 mg", "Cefixime 100 mg");
        assert!((0.0..0.5).contains(&s), "unrelated drugs scored {s}");
    }

    #[test]
    fn empty_side_never_fabricates_a_match() {
        assert_eq!(similarity("", "Amoxicillin"), 0.0);
        assert_eq!(similarity("500 mg", "Amoxicillin"), 0.0);
    }

    #[test]
    fn auto_threshold_sits_between_scores() {
        assert!(similarity("Omeprazole 20 mg", "Omeprazole 20mg") >= AUTO_MATCH_THRESHOLD);
        assert!(similarity("Paracetamol + Codeine", "Paracetamol") < AUTO_MATCH_THRESHOLD);
    }

    // ── Levenshtein ───────────────────────────────────────────────────

    #[test]
    fn levenshtein_distances() {
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("amoxicillin", "amoxicillin"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
    }
}
