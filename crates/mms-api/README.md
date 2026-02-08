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
    "profile_picture_url": "https://example.com/profile.jpg",
    "native_language": "es",
    "learning_language": "en"
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
      "profile_picture_url": "https://example.com/profile.jpg",
      "native_language": "es",
      "learning_language": "en"
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

- `GET /v1/users/me/dashboard` - Get user dashboard stats and activity
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
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

  - **Streak Calculation:** Streaks are automatically computed via a database function (`calculate_and_update_streak`) after each review. The function counts consecutive days with review activity, updating both `current_streak_days` and `longest_streak_days`.
  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `PATCH /v1/users/me/password` - Change password
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Request Body:**

  ```json
  {
    "current_password": "currentpassword123",
    "new_password": "newsecurepassword123"
  }
  ```

  - **Validation:**
    - New password: 8-128 characters, must contain at least one letter and one number
    - New password must be different from current password
    - Only available for email authentication users (not OAuth)
  - **Response:** `200 OK`

  ```json
  {
    "message": "Password changed successfully"
  }
  ```

  - Sends password change confirmation email
  - **Errors:**
    - `400 Bad Request`:
      - "Password must be at least 8 characters long"
      - "Password must be at most 128 characters long"
      - "Password must contain at least one letter and one number"
      - "Password changes are only available for email authentication users"
      - "New password must be different from current password"
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - "Password authentication not available for this account"
      - "Current password is incorrect"
      - JWT verification errors (expired, invalid signature, etc.)
    - `404 Not Found`:
      - "User not found"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database or bcrypt error)
  - **Rate Limit:** 10 req/s (General tier)

- `PATCH /v1/users/me/username` - Change username
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Request Body:**

  ```json
  {
    "username": "newusername"
  }
  ```

  - **Validation:**
    - Username: 3-30 characters, alphanumeric + underscores/hyphens
  - **Response:** `200 OK`

  ```json
  {
    "message": "Username changed successfully",
    "username": "newusername"
  }
  ```

  - **Errors:**
    - `400 Bad Request`:
      - "Username cannot be empty"
      - "Username must be at least 3 characters long"
      - "Username must be at most 30 characters long"
      - "Username can only contain letters, numbers, underscores, and hyphens"
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - JWT verification errors (expired, invalid signature, etc.)
    - `409 Conflict`:
      - "Username is already taken"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `PATCH /v1/users/me/language-preferences` - Update language preferences
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Request Body:**

  ```json
  {
    "native_language": "es",
    "learning_language": "en"
  }
  ```

  - **Validation:**
    - Both fields required
    - Must be valid ISO 639-1 language codes (e.g., "en", "es", "fr")
  - **Response:** `200 OK`

  ```json
  {
    "message": "Language preferences updated successfully",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com",
      "profile_picture_url": "https://example.com/profile.jpg",
      "native_language": "es",
      "learning_language": "en"
    }
  }
  ```

  - **Errors:**
    - `400 Bad Request`:
      - "Language code cannot be empty"
      - "Invalid language code: '{code}'. Must be a valid ISO 639-1 code (e.g., 'en', 'es', 'fr')"
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `DELETE /v1/users/me` - Delete user account
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
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

  - If the email was already verified, returns: `"Email verification processed successfully."`
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
  - **Query Parameters:**
    - `limit` (optional) - Number of results (default: 50, min: 1, max: 100)
    - `offset` (optional) - Number of results to skip (default: 0)
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
  - **Query Parameters:**
    - `limit` (optional) - Number of results (default: 50, min: 1, max: 100)
    - `offset` (optional) - Number of results to skip (default: 0)
  - **Response:** `200 OK` (same structure as above)
  - **Errors:**
    - `400 Bad Request`:
      - "Language code cannot be empty"
      - "Invalid language code: '{code}'. Must be a valid ISO 639-1 code (e.g., 'en', 'es', 'fr')"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/roadmaps/{roadmap_id}/nodes` - Get roadmap structure (public, no user progress)
  - **Path Parameters:**
    - `roadmap_id` - UUID of the roadmap
  - **Response:** `200 OK`

  ```json
  {
    "roadmap": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Spanish to English Learning Path",
      "description": "A comprehensive roadmap for learning English from Spanish",
      "language_from": "es",
      "language_to": "en",
      "total_nodes": 10,
      "completed_nodes": 0,
      "progress_percentage": 0.0
    },
    "nodes": [
      {
        "node_id": "770e8400-e29b-41d4-a716-446655440000",
        "parent_node_id": null,
        "pos_x": 100,
        "pos_y": 50,
        "deck_id": "880e8400-e29b-41d4-a716-446655440000",
        "deck_title": "Basic Greetings",
        "deck_description": "Learn common greetings and introductions",
        "total_cards": 20,
        "mastered_cards": 0,
        "cards_due_today": 0,
        "total_practices": 0,
        "last_practiced_at": null,
        "progress_percentage": 0.0
      }
    ]
  }
  ```

  - **Errors:**
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

- `GET /v1/roadmaps/{roadmap_id}/progress` - Get roadmap with user progress
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `roadmap_id` - UUID of the roadmap
  - **Response:** `200 OK`

  ```json
  {
    "roadmap": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Spanish to English Learning Path",
      "description": "A comprehensive roadmap for learning English from Spanish",
      "language_from": "es",
      "language_to": "en",
      "total_nodes": 10,
      "completed_nodes": 3,
      "progress_percentage": 30.0
    },
    "nodes": [
      {
        "node_id": "770e8400-e29b-41d4-a716-446655440000",
        "parent_node_id": null,
        "pos_x": 100,
        "pos_y": 50,
        "deck_id": "880e8400-e29b-41d4-a716-446655440000",
        "deck_title": "Basic Greetings",
        "deck_description": "Learn common greetings and introductions",
        "total_cards": 20,
        "mastered_cards": 15,
        "cards_due_today": 3,
        "total_practices": 45,
        "last_practiced_at": "2024-01-15T10:30:00Z",
        "progress_percentage": 75.5
      }
    ]
  }
  ```

  - **Progress Percentage Calculation:**
    - Each flashcard can contribute 0-10 points based on performance: `max(0, times_correct - times_wrong)`
    - Deck progress: `(sum of card points) / (total_cards * 10) * 100`
    - Example: A deck with 20 cards where each card has been answered correctly 5 times and wrong 2 times (score of 3):
      - Total points: 20 cards x 3 points = 60
      - Max points: 20 cards x 10 = 200
      - Progress: (60 / 200) x 100 = 30%
    - Roadmap progress is calculated as the percentage of completed nodes (where all cards are mastered)
  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

## Decks

- `GET /v1/decks/{deck_id}/practice` - Get practice session cards for a deck
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `deck_id` - UUID of the deck
  - **Query Parameters:**
    - `limit` (optional) - Number of cards to return (default: 20, min: 1, max: 50)
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
      - JWT verification errors (expired, invalid signature, etc.)
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error)
  - **Rate Limit:** 10 req/s (General tier)

## Practice

- `POST /v1/practice/{flashcard_id}/review` - Submit a flashcard review
  - **Authentication:** Requires valid JWT (cookie or Bearer token)
  - **Path Parameters:**
    - `flashcard_id` - UUID of the flashcard
  - **Request Body:**

  ```json
  {
    "user_answer": "Hello",
    "deck_id": "880e8400-e29b-41d4-a716-446655440000"
  }
  ```

  - **Response:** `200 OK`

  ```json
  {
    "is_correct": true,
    "correct_answer": "Hello"
  }
  ```

  - **Backend Processing:**
    - Validates the flashcard belongs to the specified deck (prevents deck progress corruption)
    - Rejects reviews if the card is not yet due (`next_review_at` is in the future) without revealing the answer
    - Validates the user's answer against the flashcard's correct translation
    - Answer validation is accent-insensitive, case-insensitive, and ignores special characters
    - Handles ligature normalization: German eszett (ß → ss), French/Latin ligatures (æ → ae, œ → oe)
    - Computes the next review date using SRS (Spaced Repetition System) algorithm based on score
    - Tracks mastery transitions: sets `mastered_at` when score reaches threshold, increments `total_cards_learned` on first mastery
    - All updates are performed atomically within a single database transaction:
      - Updates user's card progress (times_correct/times_wrong, mastered_at)
      - Refreshes deck progress (mastered_cards, progress_percentage)
      - Records user activity for the day
      - Increments total review count (and total_cards_learned if newly mastered)
      - Recalculates user streak (consecutive practice days)
  - **SRS Algorithm:**
    - Score is calculated as: `times_correct - times_wrong`
    - Uses exponential doubling with aggressive early practice
    - Hour-based intervals for early learning, transitioning to days
    - Next review intervals based on score:
      - Score <= 0: 2 hours (immediate retry)
      - Score 1: 4 hours
      - Score 2: 8 hours
      - Score 3: 1 day
      - Score 4: 2 days
      - Score 5: 5 days
      - Score 6: 10 days
      - Score 7: 20 days (~3 weeks)
      - Score 8: 40 days (~6 weeks)
      - Score 9: 60 days (2 months)
      - Score >= 10: 90 days (3 months, mastered)
  - **Translation Validation:**
    - Both the user's answer and correct translation are normalized:
      - Ligatures expanded: ß → ss, æ → ae, œ → oe
      - Accents removed via Unicode NFD decomposition (e.g., "cafe" matches "cafe")
      - Converted to lowercase (e.g., "Hello" matches "hello")
      - Non-alphanumeric characters removed (e.g., "Hello!" matches "Hello")
      - Whitespace normalized
    - Normalized strings must match exactly
  - **Errors:**
    - `401 Unauthorized`:
      - "Not authenticated" (missing auth token cookie)
      - "Failed to read cookies"
      - "Invalid user ID in token"
      - JWT verification errors (expired, invalid signature, etc.)
    - `422 Unprocessable Entity`:
      - "Flashcard does not belong to the specified deck"
      - "This card is not due for review yet"
    - `500 Internal Server Error`:
      - "An internal error occurred. Please try again later." (database error or flashcard not found)
  - **Rate Limit:** 10 req/s (General tier)

## Rate Limiting

The API implements three tiers of rate limiting:

| Tier | Rate Limit | Burst | Endpoints |
| ------ | ------------ | ------- | ----------- |
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
