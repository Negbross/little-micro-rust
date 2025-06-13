use crate::controllers::post_controller::*;
use crate::controllers::user_controller::*;
use crate::utils::AppState;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use crate::controllers::book_controller::{create_book, get_book, list_books};
use crate::controllers::category_controller::{create_category, list_categories};
use crate::controllers::file_upload_controller::upload;

pub fn routes(state: AppState) -> Router {
    let book_routes = Router::new()
        .route("/", get(list_books))
        .route("/{title}", get(get_book))
        .route("/upload", post(create_book));

    Router::new()
        .route("/", get(|| async { "hello world" }))
        .route("/posts", get(list_posts).post(create_post))
        .route(
            "/post/{slug}",
            get(get_post).put(update_post).delete(delete_post),
        )
        .route("/users", get(list_users).post(create_user))
        .route("/users/creds", post(get_user_credentials))
        .route("/categories", post(create_category).get(list_categories))
        .route("/upload", post(upload))
        .nest("/book", book_routes)
        // Layer
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state))
}

pub fn handle_error() -> Router {
    Router::new().fallback(get(not_found_error("404")))
}

/* Map any error into a `500 Internal Server Error` */
pub fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: ToString,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

// Map str into a `404 Internal Server Error`
pub fn not_found_error(msg: &str) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("{msg:?}"))
}
