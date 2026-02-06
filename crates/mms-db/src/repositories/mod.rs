// All repository functions are generic over `E: Executor<'e, Database = Postgres>`
// so they accept both a `&PgPool` (direct query) and a `&mut Transaction` (atomic operations).

pub mod auth;
pub mod deck;
pub mod practice;
pub mod roadmap;
pub mod token;
pub mod user;
