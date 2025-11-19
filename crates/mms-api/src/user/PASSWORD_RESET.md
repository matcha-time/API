# Password Reset Feature

This document describes the password reset functionality implemented in the Matcha Time API.

## Overview

The password reset feature allows users who have registered with email/password authentication to reset their password via email. This is a secure, token-based system that expires after 1 hour.

## Architecture

### Database

A new table `password_reset_tokens` stores reset tokens securely:

```sql
CREATE TABLE password_reset_tokens (
    id            UUID PRIMARY KEY,
    user_id       UUID REFERENCES users(id),
    token_hash    TEXT NOT NULL UNIQUE,
    expires_at    TIMESTAMPTZ NOT NULL,
    used_at       TIMESTAMPTZ,
    created_at    TIMESTAMPTZ DEFAULT NOW()
);
```

**Security features:**
- Tokens are hashed using SHA-256 before storage
- Only one active (unused) token per user at a time
- Tokens expire after 1 hour
- Tokens are marked as used after successful password reset

### API Endpoints

#### 1. Request Password Reset

**Endpoint:** `POST /users/request-password-reset`

**Request Body:**
```json
{
  "email": "user@example.com"
}
```

**Response:**
```json
{
  "message": "If an account exists with that email, a password reset link has been sent."
}
```

**Behavior:**
- Always returns success (prevents email enumeration)
- Only sends emails to users with email/password auth (not OAuth users)
- Invalidates any existing unused tokens for the user
- Creates a new token that expires in 1 hour
- Sends a password reset email with the token

**Security notes:**
- Response doesn't reveal if email exists or not
- Only works for `auth_provider='email'` users
- Validates email format before processing

#### 2. Reset Password

**Endpoint:** `POST /users/reset-password`

**Request Body:**
```json
{
  "token": "64-character-hex-string",
  "new_password": "NewSecurePassword123"
}
```

**Response:**
```json
{
  "message": "Password has been reset successfully. You can now log in with your new password."
}
```

**Behavior:**
- Validates the new password (min 8 chars, max 128 chars, must contain letter + number)
- Verifies token is valid, unused, and not expired
- Marks token as used
- Updates user's password hash
- Only works for users with `auth_provider='email'`

**Error responses:**
- `401 Unauthorized`: Token is invalid, expired, or already used
- `400 Bad Request`: Password doesn't meet requirements

## Email Configuration

### Environment Variables

To enable email functionality, configure the following environment variables:

```bash
# Required for email functionality
SMTP_HOST=smtp.gmail.com              # Your SMTP server
SMTP_USERNAME=your-email@gmail.com     # SMTP username
SMTP_PASSWORD=your-app-password        # SMTP password or app password
SMTP_FROM_EMAIL=noreply@matcha-time.com  # From email address
SMTP_FROM_NAME="Matcha Time"           # From name
```

### Email Service Providers

The implementation uses `lettre` with TLS support. Compatible with:
- Gmail (smtp.gmail.com:587)
- SendGrid (smtp.sendgrid.net:587)
- Mailgun (smtp.mailgun.org:587)
- Amazon SES
- Any SMTP server with TLS support

### Development Mode

If email is not configured, the system will:
- Log a warning on startup
- Print password reset tokens to console instead of sending emails
- Still accept password reset requests

Example console output:
```
Warning: Email service not configured (missing SMTP environment variables)
Email service not configured. Password reset token for user@example.com: a1b2c3d4...
```

## Email Template

The password reset email includes:
- Personalized greeting with username
- Clear explanation of what happened
- Prominent "Reset Password" button
- Plain text link as fallback
- 1-hour expiration notice
- Security reminder if request wasn't made by user

**Example URL format:**
```
https://your-frontend.com/reset-password?token=<64-char-hex-token>
```

## Security Features

### Token Generation
- 32 random bytes (256 bits of entropy)
- Hex-encoded (64 characters)
- Generated using `rand` crate's cryptographically secure RNG

### Token Storage
- Tokens are hashed with SHA-256 before database storage
- Raw tokens are never stored
- Only hashed tokens can be verified

### Token Validation
- Tokens expire after 1 hour
- Tokens can only be used once
- Tokens are user-specific
- Invalid/expired tokens return generic error message

### Rate Limiting
- Protected by application-level rate limiting
- Default: 2 requests/second, burst of 100

### Email Enumeration Prevention
- Request endpoint always returns success
- Doesn't reveal if email exists in database
- Same response time regardless of email existence

## Frontend Integration

### Step 1: Request Password Reset

```typescript
async function requestPasswordReset(email: string) {
  const response = await fetch('https://api.matcha-time.com/users/request-password-reset', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email }),
  });

  const data = await response.json();
  // Always shows success message
  console.log(data.message);
}
```

### Step 2: Handle Reset Link

```typescript
// Extract token from URL query parameter
const urlParams = new URLSearchParams(window.location.search);
const token = urlParams.get('token');

async function resetPassword(token: string, newPassword: string) {
  const response = await fetch('https://api.matcha-time.com/users/reset-password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token, new_password: newPassword }),
  });

  if (response.ok) {
    const data = await response.json();
    // Redirect to login
    window.location.href = '/login';
  } else {
    const error = await response.json();
    // Show error: token expired or invalid
    alert(error.error);
  }
}
```

## Testing

### Manual Testing

1. **Request password reset:**
```bash
curl -X POST http://localhost:3000/users/request-password-reset \
  -H "Content-Type: application/json" \
  -d '{"email":"test@example.com"}'
```

2. **Check console for token** (if email not configured)

3. **Reset password:**
```bash
curl -X POST http://localhost:3000/users/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "token":"<token-from-email-or-console>",
    "new_password":"NewPassword123"
  }'
```

4. **Login with new password:**
```bash
curl -X POST http://localhost:3000/users/login \
  -H "Content-Type: application/json" \
  -d '{
    "email":"test@example.com",
    "password":"NewPassword123"
  }'
```

### Test Cases

- ✅ Request reset for existing email user
- ✅ Request reset for non-existent email (same response)
- ✅ Request reset for OAuth user (no email sent, but success response)
- ✅ Reset password with valid token
- ✅ Reset password with expired token (401 error)
- ✅ Reset password with used token (401 error)
- ✅ Reset password with weak password (400 error)
- ✅ Multiple reset requests invalidate previous tokens
- ✅ Rate limiting prevents abuse

## Database Maintenance

### Cleanup Expired Tokens

A helper function is provided to clean up expired/used tokens:

```rust
use mms_api::user::password_reset::cleanup_expired_tokens;

// Run periodically (e.g., daily cron job)
let deleted_count = cleanup_expired_tokens(&pool).await?;
println!("Cleaned up {} expired tokens", deleted_count);
```

Consider adding a scheduled job to run this cleanup weekly.

## Migration

To apply the password reset tokens table:

```bash
# The migration will be automatically applied on server startup
cargo run

# Or manually with sqlx-cli
sqlx migrate run
```

## Troubleshooting

### Email not sending

**Check:**
1. SMTP environment variables are set
2. SMTP credentials are correct
3. SMTP host/port is accessible
4. Firewall allows outbound SMTP connections
5. Check server logs for detailed error messages

### Token expired/invalid

**Common causes:**
1. Token was already used
2. More than 1 hour passed since request
3. New reset request was made (invalidates previous token)
4. Token was modified in URL

### Password validation fails

**Requirements:**
- Minimum 8 characters
- Maximum 128 characters
- At least one letter
- At least one number

## Future Enhancements

Potential improvements to consider:

1. **Email templates:** Move HTML email to template files
2. **Custom expiration:** Make token expiration time configurable
3. **Rate limiting per email:** Prevent spam to specific email addresses
4. **Password history:** Prevent reuse of recent passwords
5. **Multi-language support:** Translate emails based on user preferences
6. **SMS/2FA reset:** Alternative recovery methods
7. **Audit logging:** Track all password reset attempts
