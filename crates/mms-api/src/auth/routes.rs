use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};

use crate::ApiState;

pub fn routes(_: ApiState) -> Router<ApiState> {
    Router::new().route("/auth/google", get(google_auth))
    //.with_state(state)
}

async fn google_auth() -> impl IntoResponse {
    (StatusCode::OK, "Hello, world!")
}
