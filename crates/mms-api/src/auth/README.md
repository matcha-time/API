# Authentication Module

This module handles all authentication and authorization logic for the API, including JWT tokens, refresh tokens, OAuth (Google), and session management.

## Table of Contents

- [Overview](#overview)
- [Authentication Flow](#authentication-flow)
- [Token System](#token-system)
- [Module Structure](#module-structure)
- [API Endpoints](#api-endpoints)
- [Security Features](#security-features)

## Overview

The authentication system supports two methods:
1. **Email/Password** - Traditional authentication with email verification
2. **Google OAuth** - Social login via OpenID Connect

Both methods use a dual-token system:
- **Access Token (JWT)** - Short-lived (24 hours), used for API requests
- **Refresh Token** - Long-lived (30 days), used to obtain new access tokens

## Authentication Flow

### Email/Password Registration & Login

```
┌─────────┐                                        ┌─────────┐
│ Client  │                                        │   API   │
└────┬────┘                                        └────┬────┘
     │                                                  │
     │  POST /users/register                           │
     │  { email, username, password }                  │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Hash password (bcrypt)
     │                                                  ├─> Create user (email_verified=false)
     │                                                  ├─> Generate verification token
     │                                                  └─> Send verification email
     │                                                  │
     │  { message: "Check your email" }                │
     │<─────────────────────────────────────────────────┤
     │                                                  │
     │  GET /users/verify-email?token=xxx              │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Verify token
     │                                                  └─> Set email_verified=true
     │                                                  │
     │  { message: "Email verified" }                  │
     │<─────────────────────────────────────────────────┤
     │                                                  │
     │  POST /users/login                              │
     │  { email, password }                            │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Verify password (bcrypt)
     │                                                  ├─> Check email_verified=true
     │                                                  ├─> Generate JWT (24h expiry)
     │                                                  ├─> Generate refresh token (30d)
     │                                                  └─> Store refresh token hash in DB
     │                                                  │
     │  Set-Cookie: auth_token=xxx                     │
     │  Set-Cookie: refresh_token=yyy                  │
     │  { token, refresh_token, user }                 │
     │<─────────────────────────────────────────────────┤
     │                                                  │
```

### Google OAuth Flow

```
┌─────────┐                             ┌─────────┐                      ┌──────────┐
│ Client  │                             │   API   │                      │  Google  │
└────┬────┘                             └────┬────┘                      └────┬─────┘
     │                                       │                                │
     │  GET /auth/google                    │                                │
     ├──────────────────────────────────────>│                                │
     │                                       │                                │
     │                                       ├─> Generate PKCE challenge     │
     │                                       ├─> Generate CSRF token         │
     │                                       └─> Store in encrypted cookie   │
     │                                       │                                │
     │  Redirect to Google with              │                                │
     │  state, nonce, PKCE challenge         │                                │
     │<──────────────────────────────────────┤                                │
     │                                       │                                │
     │  User authorizes app                                                   │
     ├───────────────────────────────────────────────────────────────────────>│
     │                                       │                                │
     │  Redirect to /auth/callback?code=xxx&state=yyy                        │
     │<───────────────────────────────────────────────────────────────────────┤
     │                                       │                                │
     │  GET /auth/callback?code=xxx&state=yyy                                │
     ├──────────────────────────────────────>│                                │
     │                                       │                                │
     │                                       ├─> Verify CSRF token            │
     │                                       │                                │
     │                                       │  Exchange code with PKCE       │
     │                                       ├───────────────────────────────>│
     │                                       │                                │
     │                                       │  ID token + access token       │
     │                                       │<───────────────────────────────┤
     │                                       │                                │
     │                                       ├─> Verify ID token signature    │
     │                                       ├─> Extract user info (email)    │
     │                                       ├─> Find or create user          │
     │                                       ├─> Generate JWT (24h)           │
     │                                       ├─> Generate refresh token       │
     │                                       └─> Store refresh token hash     │
     │                                       │                                │
     │  Set-Cookie: auth_token=xxx          │                                │
     │  Set-Cookie: refresh_token=yyy       │                                │
     │  HTML with postMessage to parent     │                                │
     │<──────────────────────────────────────┤                                │
     │                                       │                                │
```

### Token Refresh Flow (Seamless UX)

```
┌─────────┐                                        ┌─────────┐
│ Client  │                                        │   API   │
└────┬────┘                                        └────┬────┘
     │                                                  │
     │  GET /protected-resource                        │
     │  Cookie: auth_token=expired_jwt                 │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Verify JWT
     │                                                  └─> JWT expired ❌
     │                                                  │
     │  401 Unauthorized                               │
     │<─────────────────────────────────────────────────┤
     │                                                  │
     │  GET /auth/refresh                              │
     │  Cookie: refresh_token=xxx                      │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Hash refresh token
     │                                                  ├─> Find in DB
     │                                                  ├─> Check expiry (30d)
     │                                                  ├─> Delete old token
     │                                                  ├─> Generate new JWT (24h)
     │                                                  ├─> Generate new refresh token
     │                                                  └─> Store new refresh token hash
     │                                                  │
     │  Set-Cookie: auth_token=new_jwt                 │
     │  Set-Cookie: refresh_token=new_token            │
     │  { token: "new_jwt" }                           │
     │<─────────────────────────────────────────────────┤
     │                                                  │
     │  Retry: GET /protected-resource                 │
     │  Cookie: auth_token=new_jwt                     │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Verify JWT
     │                                                  └─> Valid ✅
     │                                                  │
     │  200 OK { data }                                │
     │<─────────────────────────────────────────────────┤
     │                                                  │
```

### Logout Flow

```
┌─────────┐                                        ┌─────────┐
│ Client  │                                        │   API   │
└────┬────┘                                        └────┬────┘
     │                                                  │
     │  GET /auth/logout                               │
     │  Cookie: refresh_token=xxx                      │
     ├─────────────────────────────────────────────────>│
     │                                                  │
     │                                                  ├─> Hash refresh token
     │                                                  ├─> Delete from DB
     │                                                  └─> Clear cookies
     │                                                  │
     │  Set-Cookie: auth_token=; expires=past          │
     │  Set-Cookie: refresh_token=; expires=past       │
     │  { message: "Logged out" }                      │
     │<─────────────────────────────────────────────────┤
     │                                                  │
```

## Token System

### Access Token (JWT)

**Purpose**: Authorize API requests
**Lifetime**: 24 hours
**Storage**: httpOnly cookie + returned in response
**Format**:
```json
{
  "sub": "user-uuid",
  "email": "user@example.com",
  "iat": 1234567890,
  "exp": 1234654290
}
```

**Why short-lived?**
- Minimizes damage if stolen
- Forces periodic verification via refresh

### Refresh Token

**Purpose**: Obtain new access tokens without re-login
**Lifetime**: 30 days
**Storage**:
- Client: httpOnly cookie (secure, not accessible to JS)
- Server: SHA-256 hash in `refresh_tokens` table

**Security Features**:
1. **Hashed Storage** - Plain token never stored in DB
2. **Token Rotation** - Each refresh invalidates the old token and issues a new one
3. **Revocation** - Can be invalidated server-side (logout, suspicious activity)
4. **Device Tracking** - Optional device_info and ip_address fields

## Module Structure

```
auth/
├── jwt.rs              - JWT token generation and verification
├── middleware.rs       - Auth middleware (validates JWT on protected routes)
├── models.rs           - Data structures (OidcFlowData)
├── refresh_token.rs    - Refresh token logic (generate, verify, rotate, revoke)
├── routes.rs           - Auth endpoints (/auth/google, /auth/callback, /auth/refresh, /auth/logout, /auth/me)
├── service.rs          - Business logic (find_or_create_google_user)
├── validation.rs       - Input validation (email, password, username)
└── README.md           - This file
```

## API Endpoints

### Public Endpoints (No Auth Required)

#### `POST /users/register`
Register a new user with email/password.

**Request**:
```json
{
  "email": "user@example.com",
  "username": "johndoe",
  "password": "SecurePassword123!"
}
```

**Response**: `200 OK`
```json
{
  "message": "Registration successful. Please check your email to verify your account.",
  "email": "user@example.com"
}
```

#### `POST /users/login`
Login with email/password.

**Request**:
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```

**Response**: `200 OK`
```json
{
  "token": "eyJhbGciOiJIUzI1...",
  "refresh_token": "dGhpcyBpcyBh...",
  "user": {
    "id": "uuid",
    "username": "johndoe",
    "email": "user@example.com",
    "profile_picture_url": null
  }
}
```

**Cookies Set**:
- `auth_token` (httpOnly, secure, 24h expiry)
- `refresh_token` (httpOnly, secure, 30d expiry)

#### `GET /auth/google`
Initiate Google OAuth flow.

**Response**: `302 Redirect` to Google OAuth consent screen

#### `GET /auth/callback?code=xxx&state=yyy`
OAuth callback (handled by Google).

**Response**: HTML page that posts message to parent window

#### `GET /auth/refresh`
Refresh the access token using refresh token.

**Cookies Required**: `refresh_token`

**Response**: `200 OK`
```json
{
  "token": "new_jwt_token",
  "message": "Token refreshed successfully"
}
```

**Cookies Updated**:
- `auth_token` (new JWT)
- `refresh_token` (rotated token)

#### `GET /auth/logout`
Logout and revoke refresh token.

**Response**: `200 OK`
```json
{
  "message": "Logged out successfully"
}
```

**Cookies Cleared**: `auth_token`, `refresh_token`

### Protected Endpoints (Auth Required)

#### `GET /auth/me`
Get current authenticated user info.

**Headers Required**: `Cookie: auth_token=xxx`

**Response**: `200 OK`
```json
{
  "id": "uuid",
  "username": "johndoe",
  "email": "user@example.com",
  "profile_picture_url": "https://..."
}
```

## Security Features

### 1. Password Security
- **bcrypt** hashing with default cost factor (12)
- Passwords validated for minimum strength (see [validation.rs](validation.rs:11))

### 2. Email Verification
- Users must verify email before login
- Verification tokens expire after 24 hours
- Tokens are single-use (burned after verification)

### 3. JWT Security
- Signed with secret key (HS256)
- Short expiry (24h) limits exposure window
- Contains minimal claims (user_id, email)

### 4. Refresh Token Security
- **Never stored in plain text** - SHA-256 hashed in database
- **Token rotation** - Each use generates a new token, old one is revoked
- **httpOnly cookies** - Not accessible to JavaScript (XSS protection)
- **Secure flag** - Only sent over HTTPS in production
- **Revocable** - Can be invalidated server-side

### 5. CSRF Protection
- OAuth flow uses state parameter to prevent CSRF
- SameSite=Lax cookies provide additional protection

### 6. PKCE (Proof Key for Code Exchange)
- OAuth flow uses PKCE to prevent authorization code interception
- Code verifier stored in encrypted cookie during flow

### 7. Rate Limiting
(To be implemented) - Recommend adding rate limiting on:
- `/users/login` - Prevent brute force
- `/auth/refresh` - Prevent token abuse
- `/users/register` - Prevent spam

### 8. Input Validation
All user inputs validated:
- Email format (RFC 5322)
- Password strength (min length, complexity)
- Username constraints

## Frontend Integration Guide

### 1. Login/Register
```typescript
// Login
const response = await fetch('/users/login', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  credentials: 'include', // Important: send cookies
  body: JSON.stringify({ email, password })
});

const { token, refresh_token, user } = await response.json();
// Tokens are also in httpOnly cookies
```

### 2. Authenticated Requests
```typescript
// Access token is automatically sent via cookie
const response = await fetch('/protected-resource', {
  credentials: 'include'
});

if (response.status === 401) {
  // Token expired, refresh it
  await refreshToken();
  // Retry the request
}
```

### 3. Token Refresh (Automatic)
```typescript
async function refreshToken() {
  const response = await fetch('/auth/refresh', {
    credentials: 'include'
  });

  if (response.ok) {
    const { token } = await response.json();
    // New token is in cookie, optionally store in memory
    return token;
  } else {
    // Refresh failed, redirect to login
    window.location.href = '/login';
  }
}

// Axios interceptor example
axios.interceptors.response.use(
  response => response,
  async error => {
    if (error.response?.status === 401) {
      await refreshToken();
      return axios.request(error.config); // Retry
    }
    return Promise.reject(error);
  }
);
```

### 4. Logout
```typescript
await fetch('/auth/logout', {
  credentials: 'include'
});
// Redirect to login page
window.location.href = '/login';
```

## Database Schema

### `refresh_tokens` Table

```sql
CREATE TABLE refresh_tokens (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id         UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,
    device_info     TEXT,
    ip_address      TEXT,
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW(),
    last_used_at    TIMESTAMPTZ DEFAULT NOW()
);
```

**Indexes**:
- `idx_refresh_tokens_hash` - Fast token lookup
- `idx_refresh_tokens_user` - Get all tokens for a user

## Maintenance Tasks

### Cleanup Expired Tokens

Periodically run cleanup to remove expired refresh tokens:

```rust
use crate::auth::refresh_token::cleanup_expired_tokens;

// In a background job or cron task
let deleted_count = cleanup_expired_tokens(&pool).await?;
println!("Cleaned up {} expired refresh tokens", deleted_count);
```

Recommended schedule: Daily or weekly

## Common Issues & Troubleshooting

### "Invalid or expired token" on /auth/refresh
- Refresh token may have expired (30 days)
- Token may have been revoked (logout)
- User should be redirected to login

### "No refresh token found"
- Cookie not being sent (check `credentials: 'include'`)
- Cookie expired or cleared
- User needs to login again

### CORS issues with cookies
Ensure backend CORS config includes:
```rust
.allow_credentials(true)
.allow_origin(frontend_url) // Must be specific origin, not "*"
```

### Cookies not being set
- Check secure flag (disable in development)
- Verify SameSite attribute
- Ensure path is "/"

## Future Enhancements

- [ ] Rate limiting on auth endpoints
- [ ] Email-based 2FA
- [ ] Remember device (extend refresh token for trusted devices)
- [ ] Account activity log (login history)
- [ ] Suspicious activity detection
- [ ] Password reset flow improvements
- [ ] Additional OAuth providers (GitHub, Microsoft)
