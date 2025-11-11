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

#### Health
- `GET /health` → 200 OK

#### Roadmaps

##### List All Roadmaps
- **Endpoint:** `GET /roadmaps`
- **Description:** Returns a list of all roadmaps, ordered by creation date (newest first)
- **Response:** `200 OK` with JSON array of `Roadmap` objects

**Example Response:**
```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "title": "Spanish to English Learning Path",
    "description": "A comprehensive roadmap for learning English from Spanish",
    "language_from": "es",
    "language_to": "en"
  },
  {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "title": "French to English Beginner's Guide",
    "description": "Start your English journey from French",
    "language_from": "fr",
    "language_to": "en"
  }
]
```

**Data Model: Roadmap**
- `id` (UUID): Unique identifier for the roadmap
- `title` (string): Title of the roadmap
- `description` (string, nullable): Optional description of the roadmap
- `language_from` (string): Source language code (e.g., "es", "fr", "de")
- `language_to` (string): Target language code (e.g., "en")

##### Get Roadmaps by Language Pair
- **Endpoint:** `GET /roadmaps/{language_from}/{language_to}`
- **Description:** Returns all roadmaps for a specific language pair
- **Path Parameters:**
  - `language_from` (string): Source language code (e.g., "es", "fr")
  - `language_to` (string): Target language code (e.g., "en")
- **Response:** `200 OK` with JSON array of `Roadmap` objects

**Example Request:**
```
GET /roadmaps/es/en
```

**Example Response:**
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

##### Get Roadmap with User Progress
- **Endpoint:** `GET /roadmaps/{roadmap_id}/progress/{user_id}`
- **Description:** Returns all nodes in a roadmap with the user's progress for each deck
- **Path Parameters:**
  - `roadmap_id` (UUID): The ID of the roadmap
  - `user_id` (UUID): The ID of the user
- **Response:** `200 OK` with JSON array of `RoadmapNodeWithProgress` objects, ordered by position (pos_y, then pos_x)

**Example Request:**
```
GET /roadmaps/550e8400-e29b-41d4-a716-446655440000/progress/660e8400-e29b-41d4-a716-446655440000
```

**Example Response:**
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
  {
    "node_id": "770e8400-e29b-41d4-a716-446655440001",
    "pos_x": 200,
    "pos_y": 50,
    "deck_id": "880e8400-e29b-41d4-a716-446655440001",
    "deck_title": "Common Verbs",
    "total_cards": 30,
    "mastered_cards": 0,
    "cards_due_today": 0,
    "total_practices": 0,
    "last_practiced_at": null
  },
  {
    "node_id": "770e8400-e29b-41d4-a716-446655440002",
    "pos_x": 150,
    "pos_y": 150,
    "deck_id": "880e8400-e29b-41d4-a716-446655440002",
    "deck_title": "Numbers 1-100",
    "total_cards": 100,
    "mastered_cards": 50,
    "cards_due_today": 10,
    "total_practices": 120,
    "last_practiced_at": "2024-01-14T08:15:00Z"
  }
]
```

**Data Model: RoadmapNodeWithProgress**
- `node_id` (UUID): Unique identifier for the roadmap node
- `pos_x` (integer): X coordinate position of the node
- `pos_y` (integer): Y coordinate position of the node
- `deck_id` (UUID): Unique identifier for the deck associated with this node
- `deck_title` (string): Title of the deck
- `total_cards` (integer): Total number of cards in the deck
- `mastered_cards` (integer): Number of cards the user has mastered
- `cards_due_today` (integer): Number of cards due for review today
- `total_practices` (integer): Total number of practice sessions for this deck
- `last_practiced_at` (datetime, nullable): ISO 8601 timestamp of the last practice session, or `null` if never practiced

#### Users
- `GET /users` → list users
- `GET /users/:id` → get user by id
- `POST /users` → create user
- `PUT /users/:id` → update user
- `DELETE /users/:id` → delete user

#### Topics
- `GET /topics` → list topics
- `GET /topics/:id` → get topic by id
- `POST /topics` → create topic
- `PUT /topics/:id` → update topic
- `DELETE /topics/:id` → delete topic

#### Decks
- `GET /decks` → list decks
- `GET /decks/:id` → get deck by id
- `POST /decks` → create deck
- `PUT /decks/:id` → update deck
- `DELETE /decks/:id` → delete deck
- `GET /decks/topic/:topic_id` → list decks for a topic

#### Not found
- Any other path → 404 with message "The requested resource was not found"

### Notes

- Data is dummy/in-memory for now (see `crates/mms-api/src/**/model.rs`).
- Binary entry point: `bin/serv` (workspace default member).

