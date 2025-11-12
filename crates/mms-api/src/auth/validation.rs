use crate::error::ApiError;

/// Validate email format
pub fn validate_email(email: &str) -> Result<(), ApiError> {
    if email.is_empty() {
        return Err(ApiError::Auth("Email cannot be empty".to_string()));
    }

    // Basic email validation
    if !email.contains('@') || !email.contains('.') {
        return Err(ApiError::Auth("Invalid email format".to_string()));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(ApiError::Auth("Invalid email format".to_string()));
    }

    Ok(())
}

/// Validate password strength
pub fn validate_password(password: &str) -> Result<(), ApiError> {
    if password.len() < 8 {
        return Err(ApiError::Auth(
            "Password must be at least 8 characters long".to_string(),
        ));
    }

    if password.len() > 128 {
        return Err(ApiError::Auth(
            "Password must be at most 128 characters long".to_string(),
        ));
    }

    // Check for at least one letter and one number
    let has_letter = password.chars().any(|c| c.is_alphabetic());
    let has_number = password.chars().any(|c| c.is_numeric());

    if !has_letter || !has_number {
        return Err(ApiError::Auth(
            "Password must contain at least one letter and one number".to_string(),
        ));
    }

    Ok(())
}

/// Validate username
pub fn validate_username(username: &str) -> Result<(), ApiError> {
    if username.is_empty() {
        return Err(ApiError::Auth("Username cannot be empty".to_string()));
    }

    if username.len() < 3 {
        return Err(ApiError::Auth(
            "Username must be at least 3 characters long".to_string(),
        ));
    }

    if username.len() > 30 {
        return Err(ApiError::Auth(
            "Username must be at most 30 characters long".to_string(),
        ));
    }

    // Check for valid characters (alphanumeric, underscore, hyphen)
    if !username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ApiError::Auth(
            "Username can only contain letters, numbers, underscores, and hyphens".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("user@").is_err());
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
        assert!(validate_username("ab").is_err());
        assert!(validate_username("").is_err());
        assert!(validate_username("user name").is_err());
    }
}
