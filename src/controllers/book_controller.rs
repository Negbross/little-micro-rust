use std::collections::HashMap;
use std::hash::Hash;
use crate::utils::AppState;
use axum::extract::{Multipart, Path, Query, State};
use entity::book::Entity;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, Set, ColumnTrait, QueryOrder, QuerySelect, QueryTrait};
use serde_json::json;
use std::sync::Arc;
use axum::http::StatusCode;
use axum::Json;
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use futures::TryStreamExt;
use tracing::info;
use uuid::Uuid;
use entity::{book, book_category, category, post, post_category, user};
use crate::app::files::files::{path_storage, write_file};
use crate::app::files::validator::sanitize_filename;
use crate::respons::{api_response, api_response_single};
use crate::routes::{internal_error, not_found_error};

#[axum::debug_handler]
pub async fn list_books(_state: State<Arc<AppState>>, Query(params): Query<HashMap<String, String>>)
    -> Result<impl IntoResponse, (StatusCode, String)>
{
    let search = params.get("search").cloned();
    let user_filter = params.get("uploader").cloned();
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

    let mut query = book::Entity::find().find_also_related(entity::user::Entity)
        .order_by_desc(book::Column::CreatedAt);

    // Filter title
    if let Some(title) = search {
        let pattern = format!("%{}%", title);
        query = query.filter(book::Column::Title.eq(pattern));
    }

    // Filter user
    if let Some(username) = user_filter {
        if let Some(user) = entity::user::Entity::find()
            .filter(entity::user::Column::Username.eq(username))
            .one(&_state.database_connection)
            .await
            .map_err(internal_error)?
        {
            query = query.filter(book::Column::UserId.eq(user.id));
        } else {
            return Ok(api_response(Vec::<serde_json::Value>::new())); // User isn't found → kosong
        }
    }

    // Filter category (via slug)
    if let Some(slug) = category_slug {
        if let Some(cat) = entity::category::Entity::find()
            .filter(category::Column::Slug.eq(slug))
            .one(&_state.database_connection)
            .await
            .map_err(internal_error)?
        {
            query = query
                .filter(book::Column::Id.in_subquery(
                    book_category::Entity::find()
                        .select_only()
                        .column(book_category::Column::BookId)
                        .filter(book_category::Column::CategoryId.eq(cat.id))
                        .into_query(),
                ));
        } else {
            return Ok(api_response(Vec::<serde_json::Value>::new())); // Category isn't found → kosong
        }
    }

    let book_with_users = query
        .offset(offset)
        .limit(limit)
        .all(&_state.database_connection).await
        .map_err(internal_error)?;

    let mut data = Vec::new();
    for (book_model, user_opt) in book_with_users {
        // Categories
        let categories = book::Entity::find()
            .filter(book_category::Column::BookId.eq(book_model.id))
            .all(&_state.database_connection)
            .await
            .expect("Book collection does not exist")
            .into_iter()
            .map(|rel| rel.id)
            .collect::<Vec<_>>();

        // Ambil uploader (user relasi)
        // let uploader = book_model
        //     .find_related(user::Entity)
        //     .one(&_state.database_connection)
        //     .await
        //     .expect("User is banned or smth")
        //     .map(|u| json!({
        //         "id": u.id,
        //         "name": u.name,
        //     }));

        data.push(json!({
                "title": book_model.title,
                "book_file": book_model.book_file,
                "writer": book_model.writer,
                "uploader": user_opt.map(|u| u.username),
                "publisher": book_model.publisher,
                "categories": categories,
                "created_at": book_model.created_at,
                "updated_at": book_model.updated_at,
            }));
    }
    Ok(api_response(data))
}

#[axum::debug_handler]
pub async fn create_book(
    _state: State<Arc<AppState>>,
    mut payload: Multipart
) -> impl IntoResponse {
    use crate::app::files::validator::{validate_book_mime, validate_chunk_size};

    let mut title = String::new();
    let mut writer = String::new();
    let mut publisher = String::new();
    let mut file_name = String::new();
    let mut categories = Vec::new();

    let mut chunk_number = 0;
    let mut total_chunks = 0;
    let mut chunk_data = Vec::new();

    while let Some(field) = match payload.next_field().await {
        Ok(f) => f,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    } {
        match field.name().unwrap_or("") {
            "title" => title = field.text().await.unwrap_or_default(),
            "writer" => writer = field.text().await.unwrap_or_default(),
            "publisher" => publisher = field.text().await.unwrap_or_default(),
            "filename" => file_name = field.text().await.unwrap_or_default(),
            "categories" => {
                let raw = field.text().await.unwrap_or_default();
                if let Ok(parsed) = serde_json::from_str::<Vec<String>>(&raw) {
                    categories.extend(parsed);
                }
            },
            "chunkNumber" => {
                chunk_number = field.text().await.unwrap_or_default().parse().unwrap_or(0);
            }
            "totalChunks" => {
                total_chunks = field.text().await.unwrap_or_default().parse().unwrap_or(0);
            }
            "chunkData" => {
                chunk_data = field.bytes().await.unwrap_or_default().to_vec();
            }
            _ => {}
        }
    }

    // // File name with time chrono
    let timestamp = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let sanitized_name = sanitize_filename(&file_name);
    let final_file_name = format!("{}_{}", &timestamp, sanitized_name);

    if title.is_empty() || chunk_data.is_empty() {
        return (StatusCode::BAD_REQUEST, "Title and book file required").into_response();
    }

    // ✅ Validasi MIME dan size
    if let Err(e) = validate_book_mime(&chunk_data) {
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, e.to_string()).into_response();
    }

    if let Err(e) = validate_chunk_size(&chunk_data, 5) {
        return (StatusCode::PAYLOAD_TOO_LARGE, e.to_string()).into_response();
    }

    let upload_dir = path_storage(&format!("uploads/{}/chunk", timestamp));
    let chunk_path = upload_dir.join(chunk_number.to_string());
    let file_path = format!("/uploads/books/{}", final_file_name);

    // Simpan chunk ke file
    if let Err(err) = tokio::fs::write(&chunk_path, chunk_data).await {
        eprintln!("Error writing chunk: {:?}", err);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // Jika ini adalah chunk terakhir, gabungkan semua
    if chunk_number + 1 == total_chunks {
        let chunk_relative_path = format!("uploads/{}/chunk", timestamp);

        return match write_file(&chunk_relative_path, &final_file_name, total_chunks, Some("books"))
            .await {
            Ok(file_out) => {
                use sea_orm::ActiveValue::Set;
                let b = book::ActiveModel {
                    title: Set(title),
                    writer: Set(writer),
                    publisher: Set(publisher),
                    book_file: Set(file_path),
                    created_at: Set(Utc::now()),
                    ..Default::default()
                }.insert(&_state.database_connection)
                    .await
                    .expect("Error insert");
                attach_categories_to_book(
                    &_state.database_connection,
                    b.id,
                    categories
                ).await.expect("Error attach");

                info!("Book uploaded: {}, Hash: {}", final_file_name, file_out.1);
                (StatusCode::OK, format!("Hash: {}", file_out.1)).into_response()
            }
            Err((status, msg)) => {
                (status, msg).into_response()
            }
        }
    }

    StatusCode::OK.into_response()
}

pub async fn get_book(
    _state: State<Arc<AppState>>,
    Path(title): Path<String>,
) -> impl IntoResponse {
    let book = book::Entity::find()
        .filter(book::Column::Title.eq(title))
        .one(&_state.database_connection)
        .await
        .expect("Not found");

    match book {
        Some(book) => (StatusCode::OK, api_response_single(book)).into_response(),
        None => not_found_error("Book not found").into_response(),
    }
}

pub async fn attach_categories_to_book(
    state: &DatabaseConnection,
    book_id: Uuid,
    categories: Vec<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    for name in categories {
        let cat = category::Entity::find()
            .filter(category::Column::Name.eq(name.clone()))
            .one(state)
            .await
            .expect("Error finding category");
        let Some(cat) = cat else {
            return Err(not_found_error("category not found"));
        };

        // Buat relasi book - category
        let rel = book_category::ActiveModel {
            book_id: Set(book_id),
            category_id: Set(cat.id),
        };

        rel.insert(state).await.expect("Error insert");
    }

    Ok((StatusCode::OK, "Categories saved".to_string()))
}