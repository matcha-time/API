//! Answer normalization for vocabulary comparison.
//!
//! This module handles the critical task of comparing user-typed answers against
//! correct translations. It must be lenient on accents, casing, and whitespace
//! while still being strict enough to verify actual vocabulary knowledge.

use unicode_normalization::UnicodeNormalization;

/// Normalize a string for vocabulary answer comparison.
///
/// Applies the following transformations in order:
/// 1. Lowercase
/// 2. Language-specific ligature expansion (e.g. `ß` -> `ss`)
/// 3. Unicode NFD decomposition to separate base characters from combining marks
/// 4. Strip combining marks (accents, diacritics) — keeps base letters and digits
/// 5. Collapse and trim whitespace
///
/// This means `"café"` and `"cafe"` match, `"Über"` and `"uber"` match,
/// but `"chat"` and `"chats"` do not.
pub fn normalize_for_comparison(s: &str) -> String {
    s.to_lowercase()
        .replace('ß', "ss")
        .replace('æ', "ae")
        .replace('œ', "oe")
        .nfd()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Basic behavior ---

    #[test]
    fn test_identity() {
        assert_eq!(normalize_for_comparison("hello"), "hello");
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(
            normalize_for_comparison("Hello"),
            normalize_for_comparison("hello")
        );
        assert_eq!(
            normalize_for_comparison("WORLD"),
            normalize_for_comparison("world")
        );
    }

    #[test]
    fn test_whitespace_collapse() {
        assert_eq!(
            normalize_for_comparison("hello   world"),
            normalize_for_comparison("hello world")
        );
        assert_eq!(
            normalize_for_comparison("  hello  "),
            normalize_for_comparison("hello")
        );
    }

    #[test]
    fn test_empty_and_whitespace_only() {
        assert_eq!(normalize_for_comparison(""), "");
        assert_eq!(normalize_for_comparison("   "), "");
    }

    // --- French ---

    #[test]
    fn test_french_accents() {
        assert_eq!(
            normalize_for_comparison("café"),
            normalize_for_comparison("cafe")
        );
        assert_eq!(
            normalize_for_comparison("résumé"),
            normalize_for_comparison("resume")
        );
        assert_eq!(
            normalize_for_comparison("naïve"),
            normalize_for_comparison("naive")
        );
        assert_eq!(
            normalize_for_comparison("garçon"),
            normalize_for_comparison("garcon")
        );
        assert_eq!(
            normalize_for_comparison("hôtel"),
            normalize_for_comparison("hotel")
        );
    }

    #[test]
    fn test_french_ligature_oe() {
        assert_eq!(
            normalize_for_comparison("cœur"),
            normalize_for_comparison("coeur")
        );
        assert_eq!(
            normalize_for_comparison("œuf"),
            normalize_for_comparison("oeuf")
        );
    }

    // --- German ---

    #[test]
    fn test_german_eszett() {
        assert_eq!(
            normalize_for_comparison("Straße"),
            normalize_for_comparison("strasse")
        );
        assert_eq!(
            normalize_for_comparison("groß"),
            normalize_for_comparison("gross")
        );
    }

    #[test]
    fn test_german_umlauts() {
        assert_eq!(
            normalize_for_comparison("über"),
            normalize_for_comparison("uber")
        );
        assert_eq!(
            normalize_for_comparison("schön"),
            normalize_for_comparison("schon")
        );
        assert_eq!(
            normalize_for_comparison("Mädchen"),
            normalize_for_comparison("madchen")
        );
    }

    // --- Spanish ---

    #[test]
    fn test_spanish_accents() {
        assert_eq!(
            normalize_for_comparison("español"),
            normalize_for_comparison("espanol")
        );
        assert_eq!(
            normalize_for_comparison("canción"),
            normalize_for_comparison("cancion")
        );
        assert_eq!(
            normalize_for_comparison("está"),
            normalize_for_comparison("esta")
        );
    }

    #[test]
    fn test_spanish_ene() {
        // ñ decomposes to n + combining tilde via NFD, so it matches "n"
        assert_eq!(
            normalize_for_comparison("niño"),
            normalize_for_comparison("nino")
        );
        assert_eq!(
            normalize_for_comparison("año"),
            normalize_for_comparison("ano")
        );
    }

    #[test]
    fn test_spanish_inverted_punctuation() {
        // Inverted punctuation should be stripped (not alphanumeric)
        assert_eq!(
            normalize_for_comparison("¡Hola!"),
            normalize_for_comparison("hola")
        );
        assert_eq!(
            normalize_for_comparison("¿Cómo estás?"),
            normalize_for_comparison("como estas")
        );
    }

    // --- Scandinavian ---

    #[test]
    fn test_scandinavian_ae_ligature() {
        assert_eq!(
            normalize_for_comparison("Pair of æ"),
            normalize_for_comparison("pair of ae")
        );
    }

    // --- Punctuation stripping ---

    #[test]
    fn test_punctuation_stripped() {
        assert_eq!(
            normalize_for_comparison("it's"),
            normalize_for_comparison("its")
        );
        assert_eq!(
            normalize_for_comparison("l'homme"),
            normalize_for_comparison("lhomme")
        );
        assert_eq!(
            normalize_for_comparison("well-known"),
            normalize_for_comparison("wellknown")
        );
    }

    // --- Things that should NOT match ---

    #[test]
    fn test_different_words_do_not_match() {
        assert_ne!(
            normalize_for_comparison("chat"),
            normalize_for_comparison("chats")
        );
        assert_ne!(
            normalize_for_comparison("cat"),
            normalize_for_comparison("car")
        );
        assert_ne!(
            normalize_for_comparison("le"),
            normalize_for_comparison("la")
        );
    }

    #[test]
    fn test_word_order_matters() {
        assert_ne!(
            normalize_for_comparison("bon jour"),
            normalize_for_comparison("jour bon")
        );
    }

    // --- Numbers ---

    #[test]
    fn test_numbers_preserved() {
        assert_eq!(normalize_for_comparison("42"), "42");
        assert_eq!(
            normalize_for_comparison("Route 66"),
            normalize_for_comparison("route 66")
        );
    }

    // --- Multi-word phrases ---

    #[test]
    fn test_multi_word_phrases() {
        assert_eq!(
            normalize_for_comparison("Buenos días"),
            normalize_for_comparison("buenos dias")
        );
        assert_eq!(
            normalize_for_comparison("S'il vous plaît"),
            normalize_for_comparison("sil vous plait")
        );
    }
}
