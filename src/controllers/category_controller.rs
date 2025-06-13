use std::collections::HashMap;
use std::sync::Arc;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityOrSelect, EntityTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::json;
use entity::category;
use crate::respons::{api_response, api_response_single};
use crate::routes::internal_error;
use crate::utils::{slugify, AppState};

#[derive(Deserialize)]
pub struct CategoryReq {
    name: String,
}

#[axum::debug_handler]
pub async fn create_category(
    _state: State<Arc<AppState>>,
    Json(payload): Json<CategoryReq>
) -> impl IntoResponse {
    let slug = slugify(&payload.name);
    let category = category::ActiveModel {
        name: Set(payload.name.to_owned()),
        slug: Set(slug.to_owned()),
        ..Default::default()
    };

    match category.insert(&_state.database_connection).await {
        Ok(inserted_category) => Ok((StatusCode::CREATED, api_response_single(inserted_category))),
        Err(err) => Err(internal_error(err))
    }
}

#[axum::debug_handler]
pub async fn list_categories(
    _state: State<Arc<AppState>>,
    Query(query): Query<HashMap<String, String>>
) -> impl IntoResponse {
    let keyword = query.get("s").cloned();

    let mut find = category::Entity::find();

    if let Some(kw) = keyword {
        let pattern = format!("%{}%", kw);
        find = find.filter(
            category::Column::Name.like(pattern)
        );
    };

    let category = find
        .all(&_state.database_connection)
        .await
        .map_err(|e| internal_error(e.to_string()))
        .expect("Error finding categories");

    let category = category
        .into_iter()
        .map(|category|
            json!({
            "name": category.name,
            "slug": category.slug,
        })).collect::<Vec<_>>();
    api_response(category)
}