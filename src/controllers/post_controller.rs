use crate::utils::AppState;
use crate::respons::{api_response, api_response_single};
use crate::routes::{internal_error, not_found_error};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use entity::post::ActiveModel;
use entity::prelude::Post;
use sea_orm::{ActiveModelTrait, EntityOrSelect, EntityTrait, IntoActiveModel, Set};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct PostReq {
    title: String,
    text: String,
}

#[axum::debug_handler]
pub async fn list_posts(state: State<Arc<AppState>>) -> impl IntoResponse {
    let posts = entity::post::Entity
        .select()
        .all(&state.database_connection)
        .await
        .unwrap();

    let posts = posts
        .into_iter()
        .map(|post| {
            json!({
                "title": post.title,
                "text": post.text,
            })
        })
        .collect::<Vec<_>>();

    api_response(posts)
}

#[axum::debug_handler]
pub async fn create_post(
    _state: State<Arc<AppState>>,
    Json(form): Json<PostReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let post = ActiveModel {
        title: Set(form.title.to_owned()),
        text: Set(form.text.to_owned()),
        ..Default::default()
    };

    match post.insert(&_state.database_connection).await {
        Ok(inserted) => Ok((StatusCode::CREATED, api_response_single(inserted))),
        Err(error) => Err(internal_error(error)),
    }
}

#[axum::debug_handler]
pub async fn update_post(
    _state: State<Arc<AppState>>,
    id: Path<i32>,
    Json(form): Json<PostReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match Post::find_by_id(*id).one(&_state.database_connection).await {
        Ok(Some(old_post)) => {
            let mut post = old_post.into_active_model();
            post.title = Set(form.title.clone());
            post.text = Set(form.text.clone());

            match post.update(&_state.database_connection).await {
                Ok(updated) => Ok(api_response_single(updated)),
                Err(error) => Err(internal_error(error)),
            }
        }
        Ok(None) => Err(not_found_error("No record yet.")),
        Err(error) => Err(internal_error(error)),
    }
}

#[axum::debug_handler]
pub async fn delete_post(
    _state: State<Arc<AppState>>,
    id: Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match Post::find_by_id(*id).one(&_state.database_connection).await {
        Ok(Some(post)) => {
            match post
                .into_active_model()
                .delete(&_state.database_connection)
                .await
            {
                Ok(_) => Ok(api_response_single(
                    json!({ "message": "Post deleted successfully" }),
                )),
                Err(error) => Err(internal_error(error)),
            }
        }
        Ok(None) => Err(not_found_error("Post not found")),
        Err(error) => Err(internal_error(error)),
    }
}

#[axum::debug_handler]
pub async fn get_post(
    _state: State<Arc<AppState>>,
    id: Path<i32>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match Post::find_by_id(*id).one(&_state.database_connection).await {
        Ok(post) => Ok((StatusCode::OK, api_response_single(post))),
        Err(error) => Err(internal_error(error)),
    }
}
