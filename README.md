## MemoryCards API (Rust + Axum)

A very small Axum service that exposes dummy in-memory endpoints for users, topics, and decks. Useful as a starting point and for local development.

### Run locally

- Requirements: Rust toolchain
- Start the server on port 3000:

```bash
cargo run
```

Server listens on `http://localhost:3000`.

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

