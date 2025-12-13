use crate::error::ApiError;
use validator::ValidateEmail;

/// Validate email format using the validator crate
pub fn validate_email(email: &str) -> Result<(), ApiError> {
    if email.is_empty() {
        return Err(ApiError::Validation("Email cannot be empty".to_string()));
    }

    // Use the validator crate for proper email validation
    if !email.validate_email() {
        return Err(ApiError::Validation("Invalid email format".to_string()));
    }

    Ok(())
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::Validation(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    if password.len() > 128 {
        return Err(ApiError::Validation(
            "Password must be at most 128 characters long".to_string(),
        ));
    }

    // Check for at least one letter and one number
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());

    if !has_letter || !has_number {
        return Err(ApiError::Validation(
            "Password must contain at least one letter and one number".to_string(),
        ));
    }

    Ok(())
}

/// Validate username
pub fn validate_username(username: &str) -> Result<(), ApiError> {
    if username.is_empty() {
        return Err(ApiError::Validation("Username cannot be empty".to_string()));
    }

    if username.len() < 3 {
        return Err(ApiError::Validation(
            "Username must be at least 3 characters long".to_string(),
        ));
    }

    if username.len() > 30 {
        return Err(ApiError::Validation(
            "Username must be at most 30 characters long".to_string(),
        ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    // This prevents XSS by rejecting any HTML/script characters
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ApiError::Validation(
            "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
        ));
    }

    Ok(())
}

/// Validate profile picture URL
/// Only allows HTTPS URLs from trusted domains or data URIs
pub fn validate_profile_picture_url(url: &str) -> Result<(), ApiError> {
    if url.is_empty() {
        return Ok(()); // Empty is fine, means no profile picture
    }

    // Check length
    if url.len() > 2048 {
        return Err(ApiError::Validation(
            "Profile picture URL is too long".to_string(),
        ));
    }

    // Must be HTTPS or data URI (for base64 images)
    if !url.starts_with("https://") && !url.starts_with("data:image/") {
        return Err(ApiError::Validation(
            "Profile picture URL must use HTTPS or be a data URI".to_string(),
        ));
    }

    // Reject URLs with dangerous patterns
    let url_lower = url.to_lowercase();
    if url_lower.contains("javascript:")
        || url_lower.contains("data:text/html")
        || url_lower.contains("<script")
        || url_lower.contains("onerror=")
        || url_lower.contains("onload=")
    {
        return Err(ApiError::Validation(
            "Profile picture URL contains invalid patterns".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        // Valid emails
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user.name@example.com").is_ok());
        assert!(validate_email("user+tag@example.co.uk").is_ok());

        // Invalid emails
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
        assert!(validate_email("user@@example.com").is_err());
        assert!(validate_email("user@domain..com").is_err());
        assert!(validate_email("user@.com").is_err());
    }

    #[test]
    fn test_validate_password() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("short1").is_err());
        assert!(validate_password("noNumbers").is_err());
        assert!(validate_password("12345678").is_err());
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("user_name").is_ok());
        assert!(validate_username("user-name").is_ok());
        assert!(validate_username("userscript").is_ok()); // Valid username
        assert!(validate_username("javascript_dev").is_ok()); // Valid username
        assert!(validate_username("ab").is_err());
        assert!(validate_username("").is_err());
        assert!(validate_username("user name").is_err());

        // XSS prevention tests - these are blocked by alphanumeric check
        assert!(validate_username("<script>alert('xss')</script>").is_err());
        assert!(validate_username("user<script>").is_err());
        assert!(validate_username("user&test").is_err());
    }

    #[test]
    fn test_validate_profile_picture_url() {
        // Valid URLs
        assert!(validate_profile_picture_url("").is_ok()); // Empty is fine
        assert!(validate_profile_picture_url("https://example.com/image.jpg").is_ok());
        assert!(validate_profile_picture_url("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==").is_ok());

        // Invalid URLs
        assert!(validate_profile_picture_url("http://example.com/image.jpg").is_err()); // HTTP not allowed
        assert!(validate_profile_picture_url("javascript:alert('xss')").is_err());
        assert!(
            validate_profile_picture_url("data:text/html,<script>alert('xss')</script>").is_err()
        );
        assert!(
            validate_profile_picture_url("https://example.com/image.jpg?onerror=alert('xss')")
                .is_err()
        );
        assert!(
            validate_profile_picture_url("https://example.com/image.jpg?onload=alert('xss')")
                .is_err()
        );

        // Too long
        let long_url = format!("https://example.com/{}", "a".repeat(2050));
        assert!(validate_profile_picture_url(&long_url).is_err());
    }
}
