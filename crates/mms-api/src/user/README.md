# User Authentication Module

Production-ready email verification and password reset system.

## Features

### ✅ Email Verification

- **Secure tokens**: SHA256 hashed, 24-hour expiry, one-time use
- **Email enumeration protection**: Generic messages, consistent timing
- **Smart registration**: Auto-resend for unverified, clear errors for verified accounts
- **Transaction safety**: Atomic user creation
- **Graceful degradation**: Email failures don't block registration

**Endpoints:**

- `POST /users/register` - Create account + send verification email
- `GET /users/verify-email?token=xxx` - Verify email
- `POST /users/resend-verification` - Resend verification email
- `POST /users/login` - Login (blocks unverified users)

### ✅ Password Reset

- **Atomic operations**: Token verification + password update in single transaction
- **Security**: SHA256 hashed tokens, 1-hour expiry, one-time use
- **Confirmation emails**: Alerts users of password changes
- **Email enumeration protection**: Generic success messages always returned

**Endpoints:**

- `POST /users/request-password-reset` - Request reset email
- `POST /users/reset-password` - Reset password with token

## Module Structure

```
user/
├── mod.rs                    - Module exports
├── routes.rs                 - API endpoints
├── email.rs                  - Email sending service
├── email_verification.rs     - Email verification logic
├── password_reset.rs         - Password reset logic
└── token.rs                  - Shared token generation/hashing
```

## Security

**Implemented:**

- Token hashing (SHA256) before database storage
- One-time use token enforcement
- Token expiration (24h verification, 1h reset)
- Email enumeration prevention
- Transaction safety for atomic operations
- Generic error messages
- Graceful email send failures

**Token Security:**

- Generated: 32 random bytes → 64 hex characters
- Stored: SHA256 hash only
- Used once: Marked with `used_at` timestamp
- Auto-cleanup: Old tokens removed by cleanup job

## Production Requirements

### 1. Rate Limiting (REQUIRED)

Implement at reverse proxy level:

- Registration: 5 requests/hour per IP
- Resend verification: 3 requests/hour per IP
- Password reset: 5 requests/hour per IP

### 2. Token Cleanup (REQUIRED)

Schedule daily cleanup:

```sql
DELETE FROM email_verification_tokens WHERE expires_at < NOW() OR used_at IS NOT NULL;
DELETE FROM password_reset_tokens WHERE expires_at < NOW() OR used_at IS NOT NULL;
```

Or use the provided functions:

```rust
email_verification::cleanup_expired_tokens(&pool).await
password_reset::cleanup_expired_tokens(&pool).await
```

### 3. Email Service

Configure SMTP in `.env`:

```env
SMTP_HOST=smtp.gmail.com
SMTP_USERNAME=your-email@gmail.com
SMTP_PASSWORD=your-app-password
SMTP_FROM_EMAIL=noreply@matcha-time.com
SMTP_FROM_NAME=Matcha Time
FRONTEND_URL=https://your-domain.com
```

**Recommendations:**

- Use dedicated service (SendGrid, AWS SES, Postmark)
- Configure SPF, DKIM, DMARC
- Monitor delivery rates
- Set up bounce handling

## Development

### Without SMTP

Tokens are logged to console:

```
Email service not configured. Verification token for user {uuid}: {token}
```

Manually verify using the logged token.

### Testing Flows

**Email Verification:**

1. Register → Email sent (or token logged)
2. Try login → Blocked with message
3. Verify email → Success
4. Login → Success

**Password Reset:**

1. Request reset → Email sent (or token logged)
2. Reset password → Success + confirmation email
3. Login with new password → Success
4. Try same token → Fails

**Edge Cases:**

- Duplicate registration (unverified) → Resends email
- Duplicate registration (verified) → Error message
- Expired token → Generic error
- Already verified email → Generic success

## Error Handling

All errors return generic messages to prevent enumeration:

```rust
// Good - prevents enumeration
"If an account exists with that email, a verification link has been sent."

// Bad - reveals user existence
"Email sent to john@example.com"
```

## Email Templates

**Verification Email:**

- Subject: "Verify Your Matcha Time Email"
- Content: Welcome message + verification link
- Expiry: "This link will expire in 24 hours"

**Password Reset Email:**

- Subject: "Reset Your Matcha Time Password"
- Content: Reset instructions + reset link
- Expiry: "This link will expire in 1 hour"

**Password Changed Email:**

- Subject: "Your Matcha Time Password Has Been Changed"
- Content: Confirmation + security notice
- Action: Link to reset if unauthorized
