# API Endpoints

All API routes are prefixed with `/v1` unless otherwise noted.

## Health & Monitoring

- `GET /health` - Health check (liveness probe)
  - **Response:** `200 OK`
  - **Rate Limit:** None

- `GET /health/ready` - Readiness check (database connectivity)
  - **Response:** `200 OK` if database is accessible
  - **Rate Limit:** None

- `GET /metrics` - Prometheus metrics export
  - **Response:** Prometheus-formatted metrics
  - **Rate Limit:** None

## Authentication

### OAuth (Google)

- `GET /v1/auth/google` - Initiate Google OAuth flow
  - **Response:** `302 Redirect` to Google OAuth consent screen with PKCE challenge
  - Sets encrypted `oidc_flow` cookie containing CSRF token, nonce, and PKCE verifier
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
    - `400 Bad Request` - Invalid CSRF token or missing OIDC flow data
    - `500 Internal Server Error` - OAuth or token exchange error
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
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User not found
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/auth/logout` - Logout current user
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Response:** `200 OK`

  ```json
  {
    "message": "Logged out successfully"
  }
  ```

  - Removes `auth_token` and `refresh_token` cookies
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/auth/refresh` - Refresh access token
  - **Authentication:** Requires valid `refresh_token` cookie
  - **Response:** `200 OK`

  ```json
  {
    "token": "new_jwt_access_token",
    "refresh_token": "new_jwt_refresh_token",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com",
      "profile_picture_url": "https://example.com/profile.jpg"
    }
  }
  ```

  - Sets new HTTP-only cookies with refreshed tokens
  - **Errors:**
    - `401 Unauthorized` - Missing or invalid refresh token
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
    - Email: Valid RFC 5322 format
    - Password: 8-72 characters
  - **Response:** `200 OK`

  ```json
  {
    "message": "Registration successful. Please check your email to verify your account.",
    "email": "john@example.com"
  }
  ```

  - Sends verification email to the user
  - **Errors:**
    - `400 Bad Request` - Validation error (invalid email, password, or username format)
    - `409 Conflict` - User already exists (duplicate email or username)
    - `500 Internal Server Error` - Server error
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
    - `401 Unauthorized` - Invalid email or password
    - `500 Internal Server Error` - Server error
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
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User not found
    - `500 Internal Server Error` - Server error
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
    - Username: 3-30 characters if provided
    - Email: Valid RFC 5322 format if provided
    - New password: 8-72 characters if provided, requires `current_password`
    - Profile picture URL: Valid HTTPS URL if provided
  - **Response:** `200 OK`

  ```json
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "newusername",
    "email": "newemail@example.com",
    "profile_picture_url": "https://example.com/profile.jpg"
  }
  ```

  - **Errors:**
    - `400 Bad Request` - Validation error
    - `401 Unauthorized` - Missing, invalid authentication token, or incorrect current password
    - `404 Not Found` - User not found
    - `409 Conflict` - Username or email already exists
  - **Rate Limit:** 10 req/s (General tier)

- `DELETE /v1/users/{user_id}` - Delete user account
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `user_id` - UUID of the user
  - **Response:** `200 OK`

  ```json
  {
    "message": "User deleted successfully"
  }
  ```

  - Permanently deletes user and all associated data
  - **Errors:**
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User not found
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/users/verify-email` - Verify email address
  - **Query Parameters:**
    - `token` - Email verification token (JWT)
  - **Response:** `200 OK`

  ```json
  {
    "message": "Email verified successfully"
  }
  ```

  - **Errors:**
    - `400 Bad Request` - Invalid or expired token
    - `404 Not Found` - User not found
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
    "message": "If that email exists and is not verified, a verification email has been sent"
  }
  ```

  - **Security:** Timing-safe (50ms delay) to prevent enumeration attacks
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
    "message": "If that email exists, a password reset link has been sent"
  }
  ```

  - **Security:** Timing-safe (50ms delay) to prevent enumeration attacks
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
    - New password: 8-72 characters
  - **Response:** `200 OK`

  ```json
  {
    "message": "Password reset successfully"
  }
  ```

  - **Errors:**
    - `400 Bad Request` - Invalid or expired token, or validation error
    - `404 Not Found` - User not found
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

  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/roadmaps/{language_from}/{language_to}` - Get roadmaps by language pair
  - **Path Parameters:**
    - `language_from` - ISO 639-1 language code (e.g., "es", "en", "fr")
    - `language_to` - ISO 639-1 language code
  - **Response:** `200 OK` (same structure as above)
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
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - Roadmap or user not found
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
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - Deck or user not found
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
  - **Errors:**
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User or flashcard not found
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

When rate limited, the API returns `429 Too Many Requests`.

## Error Responses

All errors follow a consistent JSON structure:

```json
{
  "error": "Error message describing what went wrong"
}
```

**HTTP Status Codes:**

- `400 Bad Request` - Invalid request (validation errors, malformed JSON)
- `401 Unauthorized` - Missing or invalid authentication
- `404 Not Found` - Resource not found
- `409 Conflict` - Resource already exists (duplicate email/username)
- `429 Too Many Requests` - Rate limit exceeded
- `500 Internal Server Error` - Server-side error

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
