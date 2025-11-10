## MemoryCards API (Rust + Axum)

A very small Axum service that exposes dummy in-memory endpoints for users, topics, and decks. Useful as a starting point and for local development.

### Run locally

- Requirements: Rust toolchain, Docker (for database)

#### Database Setup

1. Start PostgreSQL in Docker:

```bash
docker-compose up -d
```

This will start a PostgreSQL 16 container with:
- Database: `matcha_db`
- User: `matcha_user`
- Password: `matcha_password`
- Port: `5432`

2. Set up your environment variables. Create a `.env` file in the project root:

```env
DATABASE_URL=postgresql://matcha_user:matcha_password@localhost:5432/matcha_db
GOOGLE_CLIENT_ID=your_google_client_id
GOOGLE_CLIENT_SECRET=your_google_client_secret
REDIRECT_URL=http://localhost:3000/auth/callback
JWT_SECRET=your_jwt_secret_key_here_min_32_chars
COOKIE_SECRET=your_cookie_secret_key_here_min_32_chars
```

3. Start the server on port 3000:

```bash
cargo run
```

Server listens on `http://localhost:3000`.

The database migrations will run automatically when the server starts.

#### Stop the database

```bash
docker-compose down
```

To remove the database volume (deletes all data):

```bash
docker-compose down -v
```

### Endpoints

- Health
  - `GET /` → 200 OK

- Users
  - `GET /users` → list users
  - `GET /users/:id` → get user by id
  - `POST /users` → create user
  - `PUT /users/:id` → update user
  - `DELETE /users/:id` → delete user

- Topics
  - `GET /topics` → list topics
  - `GET /topics/:id` → get topic by id
  - `POST /topics` → create topic
  - `PUT /topics/:id` → update topic
  - `DELETE /topics/:id` → delete topic

- Decks
  - `GET /decks` → list decks
  - `GET /decks/:id` → get deck by id
  - `POST /decks` → create deck
  - `PUT /decks/:id` → update deck
  - `DELETE /decks/:id` → delete deck
  - `GET /decks/topic/:topic_id` → list decks for a topic

- Not found
  - Any other path → 404 with message "The requested resource was not found"

### Notes

- Data is dummy/in-memory for now (see `crates/mms-api/src/**/model.rs`).
- Binary entry point: `bin/serv` (workspace default member).

