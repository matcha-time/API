# API Endpoints

All API routes are prefixed with `/v1` unless otherwise noted.

## Health & Monitoring

- `GET /health` - Health check (liveness probe)
  - **Response:** `200 OK`
  - **Rate Limit:** None
  - **Errors:** None (always returns 200)

- `GET /health/ready` - Readiness check (database connectivity)
  - **Response:** `200 OK` if database is accessible
  - **Rate Limit:** None
  - **Errors:**
    - `503 Service Unavailable` - Database is not accessible

- `GET /metrics` - Prometheus metrics export
  - **Response:** `200 OK` - Prometheus-formatted metrics text
  - **Rate Limit:** None
  - **Errors:** None (always returns metrics)

## Authentication

### OAuth (Google)

- `GET /v1/auth/google` - Initiate Google OAuth flow
  - **Response:** `302 Redirect` to Google OAuth consent screen with PKCE challenge
  - Sets encrypted `oidc_flow` cookie containing CSRF token, nonce, and PKCE verifier
  - **Errors:**
    - `500 Internal Server Error` - Failed to serialize OIDC data
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/auth/callback` - OAuth callback handler
  - **Query Parameters:**
    - `code` - Authorization code from Google
    - `state` - CSRF token for validation
  - **Response:** `200 OK` - HTML page that posts message to parent window and closes popup
  - Sets HTTP-only, secure cookies:
    - `auth_token` - JWT access token
    - `refresh_token` - JWT refresh token
  - **Errors:**
    - `400 Bad Request`:
      - "No OIDC flow cookie found"
      - "Failed to parse OIDC data: {details}"
      - "Invalid CSRF token"
      - "No ID token in response"
      - "ID token verification failed: {details}"
      - "No email in ID token"
    - `500 Internal Server Error`:
      - "Token exchange failed: {details}"
      - "Email not verified"
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/auth/me` - Get current authenticated user
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Response:** `200 OK`

  ```json
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "johndoe",
    "email": "john@example.com",
    "profile_picture_url": "https://example.com/profile.jpg"
  }
  ```

  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "User not found"
      - JWT verification errors (expired, invalid signature, etc.)
  - **Rate Limit:** 10 req/s (General tier)

- `POST /v1/auth/logout` - Logout current user
  - **Authentication:** Not required (clears cookies regardless)
  - **Response:** `200 OK`

  ```json
  {
    "message": "Logged out successfully"
  }
  ```

  - Removes `auth_token` and `refresh_token` cookies
  - Revokes refresh token if present (failure is logged but doesn't prevent logout)
  - **Rate Limit:** 10 req/s (General tier)

- `POST /v1/auth/refresh` - Refresh access token
  - **Authentication:** Requires valid `refresh_token` cookie
  - **Response:** `200 OK`

  ```json
  {
    "token": "new_jwt_access_token",
    "message": "Token refreshed successfully"
  }
  ```

  - Sets new HTTP-only cookies with refreshed tokens
  - **Errors:**
    - `401 Unauthorized`:
      - "No refresh token found"
      - "Invalid or expired refresh token"
      - "Refresh token has been revoked"
      - "User account no longer exists"
      - "Email verification required. Please verify your email."
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

### Email/Password

- `POST /v1/users/register` - Register a new user
  - **Request Body:**

  ```json
  {
    "username": "johndoe",
    "email": "john@example.com",
    "password": "securepassword123"
  }
  ```

  - **Validation:**
    - Username: 3-30 characters, alphanumeric + underscores/hyphens
    - Email: Valid email format
    - Password: 8-128 characters, must contain at least one letter and one number
  - **Response:** `200 OK`

  ```json
  {
    "message": "Registration successful. Please check your email to verify your account.",
    "email": "john@example.com"
  }
  ```

  - Sends verification email to the user
  - **Errors:**
    - `400 Bad Request`:
      - "Email cannot be empty"
      - "Invalid email format"
      - "Password must be at least 8 characters long"
      - "Password must be at most 128 characters long"
      - "Password must contain at least one letter and one number"
      - "Username cannot be empty"
      - "Username must be at least 3 characters long"
      - "Username must be at most 30 characters long"
      - "Username can only contain letters, numbers, underscores, and hyphens"
    - `409 Conflict`:
      - "Registration failed. This username or email may already be in use."
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database or bcrypt error)
  - **Rate Limit:** 5 req/s (Auth tier)

- `POST /v1/users/login` - Login with email and password
  - **Request Body:**

  ```json
  {
    "email": "john@example.com",
    "password": "securepassword123"
  }
  ```

  - **Response:** `200 OK`

  ```json
  {
    "token": "jwt_access_token",
    "refresh_token": "jwt_refresh_token",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com",
      "profile_picture_url": "https://example.com/profile.jpg"
    }
  }
  ```

  - Sets HTTP-only, secure cookies:
    - `auth_token` - JWT access token
    - `refresh_token` - JWT refresh token
  - **Errors:**
    - `401 Unauthorized`:
      - "Invalid email or password" (user not found, wrong password, or no password hash)
      - "Please verify your email address before logging in. Check your inbox for the verification link."
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database or bcrypt error)
  - **Rate Limit:** 5 req/s (Auth tier)

**Note:** All authentication endpoints (registration, login, OAuth callback) set HTTP-only, secure cookies (`auth_token`, `refresh_token`) containing JWT tokens, in addition to returning them in the response body. Cookies use `SameSite=Strict` in production and `SameSite=Lax` in development.

## Users

- `GET /v1/users/{user_id}/dashboard` - Get user dashboard stats and activity
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `user_id` - UUID of the user
  - **Response:** `200 OK`

  ```json
  {
    "stats": {
      "current_streak_days": 5,
      "longest_streak_days": 10,
      "total_reviews": 150,
      "total_cards_learned": 50,
      "last_review_date": "2024-01-15"
    },
    "heatmap": [
      {
        "activity_date": "2024-01-15",
        "reviews_count": 10
      }
    ]
  }
  ```

  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "You are not authorized to access this dashboard"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `PATCH /v1/users/{user_id}` - Update user profile
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `user_id` - UUID of the user
  - **Request Body:** (all fields optional)

  ```json
  {
    "username": "newusername",
    "email": "newemail@example.com",
    "current_password": "currentpassword123",
    "new_password": "newsecurepassword123",
    "profile_picture_url": "https://example.com/profile.jpg"
  }
  ```

  - **Validation:**
    - Username: 3-30 characters, alphanumeric + underscores/hyphens if provided
    - Email: Valid email format if provided (marks email as unverified and sends verification email)
    - New password: 8-128 characters, must contain letter and number if provided, requires `current_password`
    - Profile picture URL: Valid HTTPS or data URI if provided
  - **Response:** `200 OK`

  ```json
  {
    "message": "Profile updated successfully",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "newusername",
      "email": "newemail@example.com",
      "profile_picture_url": "https://example.com/profile.jpg"
    }
  }
  ```

  - **Errors:**
    - `400 Bad Request`:
      - "Email cannot be empty"
      - "Invalid email format"
      - "Password must be at least 8 characters long"
      - "Password must be at most 128 characters long"
      - "Password must contain at least one letter and one number"
      - "Username cannot be empty"
      - "Username must be at least 3 characters long"
      - "Username must be at most 30 characters long"
      - "Username can only contain letters, numbers, underscores, and hyphens"
      - "Profile picture URL is too long"
      - "Profile picture URL must use HTTPS or be a data URI"
      - "Profile picture URL contains invalid patterns"
      - "Password changes are only available for email authentication users"
      - "Current password is required to set a new password"
      - "New password must be different from current password"
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "You are not authorized to update this profile"
      - "Password authentication not available for this account"
      - "Current password is incorrect"
      - JWT verification errors (expired, invalid signature, etc.)
    - `404 Not Found`:
      - "User not found"
    - `409 Conflict`:
      - "Username is already taken"
      - "Email is already in use"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database or bcrypt error)
  - **Rate Limit:** 10 req/s (General tier)

- `DELETE /v1/users/{user_id}` - Delete user account
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `user_id` - UUID of the user
  - **Response:** `200 OK`

  ```json
  {
    "message": "Account deleted successfully"
  }
  ```

  - Permanently deletes user and all associated data (cascades to related records)
  - Revokes all refresh tokens for the user
  - Clears auth and refresh token cookies
  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "You are not authorized to delete this account"
      - JWT verification errors (expired, invalid signature, etc.)
    - `404 Not Found`:
      - "User not found"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/users/verify-email` - Verify email address
  - **Query Parameters:**
    - `token` - Email verification token (JWT)
  - **Response:** `200 OK`

  ```json
  {
    "message": "Email verified successfully. You can now log in to your account.",
    "email": "john@example.com"
  }
  ```

  - **Errors:**
    - `400 Bad Request`:
      - JWT verification errors (invalid token, expired, invalid signature, etc.)
    - `404 Not Found`:
      - "User not found" (token valid but user doesn't exist)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `POST /v1/users/resend-verification` - Resend email verification link
  - **Request Body:**

  ```json
  {
    "email": "john@example.com"
  }
  ```

  - **Response:** `200 OK` (always returns success to prevent email enumeration)

  ```json
  {
    "message": "If an unverified account exists with that email, a verification link has been sent."
  }
  ```

  - **Security:** Timing-safe (50ms delay) to prevent enumeration attacks
  - **Errors:**
    - `400 Bad Request`:
      - "Email cannot be empty"
      - "Invalid email format"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error, but returns 200 to prevent enumeration in practice)
  - **Rate Limit:** 2 req/s (Sensitive tier)

- `POST /v1/users/request-password-reset` - Request password reset email
  - **Request Body:**

  ```json
  {
    "email": "john@example.com"
  }
  ```

  - **Response:** `200 OK` (always returns success to prevent email enumeration)

  ```json
  {
    "message": "If an account exists with that email, a password reset link has been sent."
  }
  ```

  - **Security:** Timing-safe (50ms delay) to prevent enumeration attacks
  - **Errors:**
    - `400 Bad Request`:
      - "Email cannot be empty"
      - "Invalid email format"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error, but returns 200 to prevent enumeration in practice)
  - **Rate Limit:** 2 req/s (Sensitive tier)

- `POST /v1/users/reset-password` - Reset password using reset token
  - **Request Body:**

  ```json
  {
    "token": "reset_token_string",
    "new_password": "newsecurepassword123"
  }
  ```

  - **Validation:**
    - New password: 8-128 characters, must contain at least one letter and one number
  - **Response:** `200 OK`

  ```json
  {
    "message": "Password has been reset successfully. You can now log in with your new password."
  }
  ```

  - Sends password change confirmation email
  - **Errors:**
    - `400 Bad Request`:
      - "Password must be at least 8 characters long"
      - "Password must be at most 128 characters long"
      - "Password must contain at least one letter and one number"
    - `401 Unauthorized`:
      - "Password reset failed. The token may be invalid or expired."
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database or bcrypt error)
  - **Rate Limit:** 5 req/s (Auth tier)

**Note:** User registration and login endpoints are documented in the [Authentication](#authentication) section above.

## Roadmaps

- `GET /v1/roadmaps` - List all roadmaps
  - **Response:** `200 OK`

  ```json
  [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Spanish to English Learning Path",
      "description": "A comprehensive roadmap for learning English from Spanish",
      "language_from": "es",
      "language_to": "en"
    }
  ]
  ```

  - **Errors:**
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/roadmaps/{language_from}/{language_to}` - Get roadmaps by language pair
  - **Path Parameters:**
    - `language_from` - ISO 639-1 language code (e.g., "es", "en", "fr")
    - `language_to` - ISO 639-1 language code
  - **Response:** `200 OK` (same structure as above)
  - **Errors:**
    - `400 Bad Request`:
      - "Language code cannot be empty"
      - "Invalid language code: '{code}'. Must be a valid ISO 639-1 code (e.g., 'en', 'es', 'fr')"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/roadmaps/{roadmap_id}/progress/{user_id}` - Get roadmap with user progress
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `roadmap_id` - UUID of the roadmap
    - `user_id` - UUID of the user
  - **Response:** `200 OK`

  ```json
  [
    {
      "node_id": "770e8400-e29b-41d4-a716-446655440000",
      "pos_x": 100,
      "pos_y": 50,
      "deck_id": "880e8400-e29b-41d4-a716-446655440000",
      "deck_title": "Basic Greetings",
      "total_cards": 20,
      "mastered_cards": 15,
      "cards_due_today": 3,
      "total_practices": 45,
      "last_practiced_at": "2024-01-15T10:30:00Z"
    }
  ]
  ```

  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "You are not authorized to access this roadmap progress"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

## Decks

- `GET /v1/decks/{deck_id}/practice/{user_id}` - Get practice session cards for a deck
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `deck_id` - UUID of the deck
    - `user_id` - UUID of the user
  - **Response:** `200 OK`

  ```json
  [
    {
      "id": "990e8400-e29b-41d4-a716-446655440000",
      "term": "Hola",
      "translation": "Hello",
      "times_correct": 5,
      "times_wrong": 2
    }
  ]
  ```

  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "You are not authorized to access this deck"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

## Practice

- `POST /v1/practice/{user_id}/{flashcard_id}/review` - Submit a flashcard review
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `user_id` - UUID of the user
    - `flashcard_id` - UUID of the flashcard
  - **Request Body:**

  ```json
  {
    "correct": true,
    "next_review_at": "2024-01-15T10:30:00Z",
    "deck_id": "880e8400-e29b-41d4-a716-446655440000"
  }
  ```

  - **Response:** `200 OK`
  - Updates user's review statistics and schedules next review
  - Updates user activity heatmap
  - Updates user stats (total reviews, last review date)
  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "You are not authorized to submit reviews for this user"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

## Rate Limiting

The API implements three tiers of rate limiting:

| Tier | Rate Limit | Burst | Endpoints |
|------|------------|-------|-----------|
| **General** | 10 req/s | 20 | Most endpoints (OAuth, authenticated routes, public data) |
| **Auth** | 5 req/s | 5 | `/users/register`, `/users/login`, `/users/reset-password` |
| **Sensitive** | 2 req/s | 3 | `/users/request-password-reset`, `/users/resend-verification` |

**Timing-Safe Middleware:** Sensitive endpoints include a 50ms artificial delay to prevent timing-based enumeration attacks.

**Response Headers:**

- `X-RateLimit-Limit` - Maximum requests per second
- `X-RateLimit-Remaining` - Remaining requests in current window
- `X-RateLimit-Reset` - Time when the rate limit resets

When rate limited, the API returns `429 Too Many Requests` with error message: "Rate limit exceeded. Please try again later."

## Error Responses

All errors follow a consistent JSON structure:

```json
{
  "error": "Error message describing what went wrong"
}
```

**HTTP Status Codes:**

- `400 Bad Request` - Invalid request (validation errors, malformed JSON, invalid parameters)
- `401 Unauthorized` - Missing or invalid authentication (missing token, expired token, invalid credentials)
- `404 Not Found` - Resource not found (user, roadmap, deck, flashcard not found)
- `409 Conflict` - Resource conflict (duplicate email/username)
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server-side error (database errors are masked with generic message)

## Authentication Methods

The API supports two authentication methods:

1. **Cookie-based (recommended for web browsers):**
   - HTTP-only, secure cookies: `auth_token`, `refresh_token`
   - Automatically sent with requests
   - Protected against XSS attacks

2. **Bearer token (recommended for mobile/API clients):**
   - Include JWT in `Authorization` header: `Authorization: Bearer <token>`
   - Token obtained from login/register responses

**Token Expiry:**

- Access tokens: Configured via environment (default: 15 minutes)
- Refresh tokens: Configured via environment (default: 7 days)

## CORS & Security Headers

**CORS:** Configured based on `FRONTEND_URL` environment variable

**Security Headers:**

- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Strict-Transport-Security: max-age=31536000; includeSubDomains` (production only)

**Request Tracing:**

- All requests include `X-Request-ID` header for distributed tracing
