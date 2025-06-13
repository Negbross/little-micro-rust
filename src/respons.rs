use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};

pub fn api_response<T>(items: Vec<T>) -> impl IntoResponse
where
    T: Serialize,
{
    let list = items
        .into_iter()
        .map(|item| serde_json::to_value(item).unwrap())
        .collect::<Vec<Value>>();

    let message = if list.is_empty() {
        "No data"
    } else {
        "Ok"
    };

    Json(json!({
        "data": list,
        "message": message
    }))
}

pub fn api_response_single<T>(item: T) -> impl IntoResponse
where
    T: Serialize,
{
    let value = serde_json::to_value(item).unwrap();

    Json(value)
}
