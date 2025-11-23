# API Endpoints

## Health

- `GET /health` - Health check
  - **Response:** `200 OK`

## Authentication

### OAuth (Google)

- `GET /auth/google` - Initiate Google OAuth flow
  - **Response:** Redirect to Google OAuth with PKCE challenge
  - Sets encrypted `oidc_flow` cookie with CSRF token, nonce, and PKCE verifier

- `GET /auth/callback` - OAuth callback handler
  - **Response:** `200 OK` - HTML page that posts message to parent window and closes popup
  - Sets HTTP-only `auth_token` cookie containing JWT
  - **Errors:**
    - `400 Bad Request` - Invalid CSRF token or missing OIDC flow data
    - `500 Internal Server Error` - OAuth or server error

- `GET /auth/me` - Get current authenticated user
  - **Authentication:** Requires valid JWT cookie
  - **Response:** `200 OK`

  ```json
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "username": "John Doe",
    "email": "john@example.com"
  }
  ```

  - **Errors:**
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User not found

- `GET /auth/logout` - Logout current user
  - **Response:** `200 OK`

  ```json
  {
    "message": "Logged out successfully"
  }
  ```

  - Removes `auth_token` cookie

- `GET /auth/refresh` - Refresh access token
  - **Authentication:** Requires valid refresh token cookie
  - **Response:** `200 OK`
  - Sets new HTTP-only `auth_token` cookie containing refreshed JWT
  - **Errors:**
    - `401 Unauthorized` - Missing or invalid refresh token

### Email/Password

- `POST /users/register` - Register a new user
  - **Request Body:**

  ```json
  {
    "username": "johndoe",
    "email": "john@example.com",
    "password": "securepassword123"
  }
  ```

  - **Response:** `200 OK`

  ```json
  {
    "token": "jwt_token_string",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com"
    }
  }
  ```

  - Sets HTTP-only `auth_token` cookie containing JWT
  - **Errors:**
    - `400 Bad Request` - Validation error (invalid email, password, or username format)
    - `409 Conflict` - User already exists (duplicate email or username)
    - `500 Internal Server Error` - Server error

- `POST /users/login` - Login with email and password
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
    "token": "jwt_token_string",
    "user": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "username": "johndoe",
      "email": "john@example.com"
    }
  }
  ```

  - Sets HTTP-only `auth_token` cookie containing JWT
  - **Errors:**
    - `401 Unauthorized` - Invalid email or password
    - `500 Internal Server Error` - Server error

**Note:** All authentication endpoints (registration, login, OAuth callback) set an HTTP-only cookie (`auth_token`) containing the JWT token, in addition to returning it in the response body. The JWT is also used for authenticating protected endpoints.

## Roadmaps

- `GET /roadmaps` - List all roadmaps
  - **Response:** `200 OK`

  ```json
  [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "title": "Spanish to English Learning Path",
      "description": "A comprehensive roadmap",
      "language_from": "es",
      "language_to": "en"
    },
    // ...
  ]
  ```

- `GET /roadmaps/{language_from}/{language_to}` - Get roadmaps by language pair
  - **Response:** `200 OK` (same as above)

- `GET /roadmaps/{roadmap_id}/progress/{user_id}` - Get roadmap with user progress
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
    },
    // ...
  ]
  ```

## Users

- `GET /users/{user_id}/dashboard` - Get user dashboard stats and activity
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
      },
      // ...
    ]
  }
  ```

  - **Errors:**
    - `404 Not Found` - User not found
    - `500 Internal Server Error` - Server error

- `PATCH /users/{user_id}` - Update user profile
  - **Authentication:** Requires valid JWT cookie
  - **Request Body:** (all fields optional)

  ```json
  {
    "username": "newusername",
    "email": "newemail@example.com",
    "password": "newsecurepassword123",
    "profile_picture_url": "https://example.com/profile.jpg"
  }
  ```

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
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User not found
    - `409 Conflict` - Username or email already exists

- `DELETE /users/{user_id}` - Delete user account
  - **Authentication:** Requires valid JWT cookie
  - **Response:** `200 OK`

  ```json
  {
    "message": "User deleted successfully"
  }
  ```

  - **Errors:**
    - `401 Unauthorized` - Missing or invalid authentication token
    - `404 Not Found` - User not found

- `GET /users/verify-email` - Verify email address
  - **Query Parameters:**
    - `token` - Email verification token
  - **Response:** `200 OK`

  ```json
  {
    "message": "Email verified successfully"
  }
  ```

  - **Errors:**
    - `400 Bad Request` - Invalid or expired token
    - `404 Not Found` - User not found

- `POST /users/resend-verification` - Resend email verification link
  - **Request Body:**

  ```json
  {
    "email": "john@example.com"
  }
  ```

  - **Response:** `200 OK`

  ```json
  {
    "message": "Verification email sent"
  }
  ```

  - **Errors:**
    - `400 Bad Request` - Invalid email or already verified
    - `404 Not Found` - User not found

- `POST /users/request-password-reset` - Request password reset email
  - **Request Body:**

  ```json
  {
    "email": "john@example.com"
  }
  ```

  - **Response:** `200 OK`

  ```json
  {
    "message": "Password reset email sent"
  }
  ```

  - **Errors:**
    - `404 Not Found` - User not found

- `POST /users/reset-password` - Reset password using reset token
  - **Request Body:**

  ```json
  {
    "token": "reset_token_string",
    "new_password": "newsecurepassword123"
  }
  ```

  - **Response:** `200 OK`

  ```json
  {
    "message": "Password reset successfully"
  }
  ```

  - **Errors:**
    - `400 Bad Request` - Invalid or expired token
    - `404 Not Found` - User not found

**Note:** User registration and login endpoints are documented in the [Authentication](#authentication) section above.

## Decks

- `GET /decks/{deck_id}/practice/{user_id}` - Get practice session cards for a deck
  - **Response:** `200 OK`

  ```json
  [
    {
      "id": "990e8400-e29b-41d4-a716-446655440000",
      "term": "Hola",
      "translation": "Hello",
      "times_correct": 5,
      "times_wrong": 2
    },
    // ...
  ]
  ```

## Practice

- `POST /practice/{user_id}/{flashcard_id}/review` - Submit a flashcard review
  - **Request Body:**

  ```json
  {
    "correct": true,
    "next_review_at": "2024-01-15T10:30:00Z",
    "deck_id": "880e8400-e29b-41d4-a716-446655440000"
  }
  ```

  - **Response:** `200 OK`
