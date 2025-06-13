use std::collections::HashMap;
use crate::utils::{slugify, AppState};
use crate::respons::{api_response, api_response_single};
use crate::routes::{internal_error, not_found_error};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use entity::post::{ActiveModel, Column};
use entity::prelude::Post;
use sea_orm::{ActiveModelTrait, EntityOrSelect, EntityTrait, IntoActiveModel, QueryFilter, Set, ColumnTrait, DatabaseConnection, DbErr, ModelTrait, TransactionTrait, QueryOrder, QuerySelect, QueryTrait};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use entity::{category, post, post_category};

#[derive(Debug, Deserialize)]
pub struct PostReq {
    title: String,
    text: String,
    categories: Vec<String>,
    uploader: String
}

#[axum::debug_handler]
pub async fn list_posts(
    state: State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let search = params.get("search").cloned();
    let user_filter = params.get("user").cloned();
    let category_slug = params.get("category").cloned();

    let page: u64 = params
        .get("page")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let limit: u64 = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    let offset = (page - 1) * limit;

    let mut query = post::Entity::find().find_also_related(entity::user::Entity)
        .order_by_desc(post::Column::CreatedAt);

    // Filter title
    if let Some(kw) = search {
        query = query.filter(post::Column::Title.contains(kw));
    }

    // Filter user
    if let Some(username) = user_filter {
        if let Some(user) = entity::user::Entity::find()
            .filter(entity::user::Column::Username.eq(username))
            .one(&state.database_connection)
            .await
            .map_err(internal_error)?
        {
            query = query.filter(post::Column::UserId.eq(user.id))
        } else {
            return Ok(api_response(Vec::<serde_json::Value>::new())); // User isn't found → kosong
        }
    };

    // Filter category (via slug)
    if let Some(slug) = category_slug {
        if let Some(cat) = entity::category::Entity::find()
            .filter(category::Column::Slug.eq(slug))
            .one(&state.database_connection)
            .await
            .map_err(internal_error)?
        {
            query = query
                .filter(post::Column::Id.in_subquery(
                    post_category::Entity::find()
                        .select_only()
                        .column(post_category::Column::PostId)
                        .filter(post_category::Column::CategoryId.eq(cat.id))
                        .into_query(),
                ));
        } else {
            return Ok(api_response(Vec::<serde_json::Value>::new())); // Category isn't found → kosong
        }
    }

    let posts_with_user = query
        .offset(offset)
        .limit(limit)
        .all(&state.database_connection).await.map_err(internal_error)?;

    let mut results = Vec::with_capacity(posts_with_user.len());

    for (post_model, user_opt) in posts_with_user {
        let categories = post_model
            .find_related(category::Entity)
            .all(&state.database_connection)
            .await.map_err(internal_error)?
            .into_iter()
            .map(|category| category.name)
            .collect::<Vec<_>>();

        results.push(json!({
            "id": post_model.id,
            "title": post_model.title,
            "slug": post_model.slug,
            "text": post_model.text,
            "username": user_opt.map(|u| u.username),
            "categories": categories,
            "created_at": post_model.created_at,
            "updated_at": post_model.updated_at,
        }));
    }

    Ok(api_response(results))
}

#[axum::debug_handler]
pub async fn create_post(
    _state: State<Arc<AppState>>,
    Json(form): Json<PostReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = &_state.database_connection;
    let slug = slugify(&form.title);
    let user = entity::user::Entity::find()
        .filter(entity::user::Column::Username.eq(form.uploader.clone()))
        .one(&_state.database_connection)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("User '{}' not found", form.uploader)))?;

    let txn = db.begin().await.map_err(internal_error)?;

    // Insert post
    let new_post = post::ActiveModel {
        title: Set(form.title.clone()),
        text: Set(form.text.clone()),
        slug: Set(slug),
        user_id: Set(user.id.clone()),
        ..Default::default()
    };
    info!("{:?}", form.categories.clone());

    let inserted_post = new_post.insert(&txn).await.map_err(internal_error)?;

    // Insert relasi ke kategori
    for name in &form.categories {
        if let Some(cat) = category::Entity::find()
            .filter(category::Column::Name.eq(name))
            .one(&txn)
            .await
            .map_err(internal_error)?
        {
            let rel = post_category::ActiveModel {
                post_id: Set(inserted_post.id.to_owned()),
                category_id: Set(cat.id),
            };

            // Jika insert relasi gagal, rollback
            if let Err(e) = rel.insert(&txn).await {
                txn.rollback().await.ok(); // best effort rollback
                return Err(internal_error(format!("Gagal menghubungkan kategori '{}': {}", name, e)));
            }
        }
        // Jika kategori tidak ditemukan → skip
    }

    // Commit
    txn.commit().await.map_err(internal_error)?;

    Ok((StatusCode::CREATED, api_response_single(inserted_post)))
    // let post = ActiveModel {
    //     title: Set(form.title.to_owned()),
    //     text: Set(form.text.to_owned()),
    //     slug: Set(slug.to_owned()),
    //     user_id: Set(user.id.clone()),
    //     ..Default::default()
    // };
    //
    // match post.insert(&_state.database_connection).await {
    //     Ok(inserted) => {
    //         // Attach categories
    //         if let Err(err) = attach_categories_to_post(
    //             &_state.database_connection,
    //             inserted.id.parse().unwrap(),
    //             form.categories.clone(),
    //         ).await {
    //             return Err(internal_error(err));
    //         }
    //
    //         Ok((StatusCode::CREATED, api_response_single(inserted)))
    //     },
    //     Err(error) => Err(internal_error(error)),
    // }
}

#[axum::debug_handler]
pub async fn update_post(
    _state: State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(form): Json<PostReq>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match Post::find()
        .filter(Column::Slug.eq(slug))
        .one(&_state.database_connection).await {
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
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match Post::find()
        .filter(Column::Slug.eq(slug))
        .one(&_state.database_connection).await {
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
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match Post::find()
        .filter(Column::Slug.eq(slug.as_str()))
        .one(&_state.database_connection).await {
        Ok(post) => Ok((StatusCode::OK, api_response_single(post))),
        Err(error) => Err(internal_error(error)),
    }
}

// pub async fn attach_categories_to_post(
//     db: &DatabaseConnection,
//     post_id: Uuid,
//     categories: Vec<String>,
// ) -> Result<(), DbErr> {
//     for name in categories {
//         if let Some(cat) = category::Entity::find()
//             .filter(category::Column::Name.eq(name.clone()))
//             .one(db)
//             .await?
//         {
//             let rel = post_category::ActiveModel {
//                 post_id: Set(post_id),
//                 category_id: Set(cat.id),
//             };
//             rel.insert(db).await?;
//         }
//     }
//     Ok(())
// }
