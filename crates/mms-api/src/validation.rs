use crate::error::ApiError;

/// ISO 639-1 language codes
const VALID_LANGUAGE_CODES: &[&str] = &[
    // NOTE: For now we will stick to a small list
    "en", // English
    "es", // Spanish
    "fr", // French
];

/// Validate ISO 639-1 language code
///
/// # Examples
/// ```
/// use mms_api::validation::validate_language_code;
///
/// assert!(validate_language_code("en").is_ok());
/// assert!(validate_language_code("invalid").is_err());
/// ```
pub fn validate_language_code(code: &str) -> Result<(), ApiError> {
    if code.is_empty() {
        return Err(ApiError::Validation(
            "Language code cannot be empty".to_string(),
        ));
    }

    // Normalize to lowercase for comparison
    let normalized = code.to_lowercase();

    if !VALID_LANGUAGE_CODES.contains(&normalized.as_str()) {
        return Err(ApiError::Validation(format!(
            "Invalid language code: '{}'. Must be a valid ISO 639-1 code (e.g., 'en', 'es', 'fr')",
            code
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_language_code() {
        // Valid codes
        assert!(validate_language_code("en").is_ok());
        assert!(validate_language_code("EN").is_ok()); // Case insensitive
        assert!(validate_language_code("es").is_ok());
        assert!(validate_language_code("fr").is_ok());

        // Invalid codes
        assert!(validate_language_code("").is_err());
        assert!(validate_language_code("xx").is_err());
        assert!(validate_language_code("invalid").is_err());
        assert!(validate_language_code("123").is_err());
    }
}
