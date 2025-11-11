## MemoryCards API (Rust + Axum)

A Rust API service built with Axum for managing language learning roadmaps, decks, and flashcards.

### Setup

**Requirements:** Rust toolchain, Docker (for database)

1. Start PostgreSQL:
```bash
docker-compose up -d
```

2. Create a `.env` file:
```env
DATABASE_URL=postgresql://matcha_user:matcha_password@localhost:5432/matcha_db
GOOGLE_CLIENT_ID=your_google_client_id
GOOGLE_CLIENT_SECRET=your_google_client_secret
REDIRECT_URL=http://localhost:3000/auth/callback
JWT_SECRET=your_jwt_secret_key_here_min_32_chars
COOKIE_SECRET=your_cookie_secret_key_here_min_32_chars
```

3. Run the server:
```bash
cargo run
```

Server runs on `http://localhost:3000`. Database migrations run automatically on startup.

### Stop Database

```bash
docker-compose down
```

To delete all data:
```bash
docker-compose down -v
```

### API Documentation

See [crates/mms-api/README.md](crates/mms-api/README.md) for endpoint documentation.

