use crate::app::hashing::hash::{hash_password, verify_password};
use crate::respons::{api_response, api_response_single};
use crate::routes::{internal_error, not_found_error};
use crate::utils::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use entity::user;
use entity::user::ActiveModel;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityOrSelect, EntityTrait, ModelTrait, QueryFilter, Set};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct UserRequest {
    username: String,
    password: String,
    name: String,
}

#[derive(Deserialize)]
pub struct UserLogin {
    username: String,
    password: String
}

#[axum::debug_handler]
pub async fn list_users(state: State<Arc<AppState>>) -> impl IntoResponse {
    let users = user::Entity
        .select()
        .all(&state.database_connection)
        .await
        .unwrap();
    let users = users
        .into_iter()
        .map(|user| {
           json!({
                "id": user.id,
                "name": user.name,
               "username": user.username,
               "password": user.password,
           })
        }).collect::<Vec<_>>();
    
    api_response(users)
}

#[axum::debug_handler]
pub async fn create_user(
    state: State<Arc<AppState>>,
    Json(user_form): Json<UserRequest>,
    
) -> impl IntoResponse {
    let password_hashing = hash_password(user_form.password.as_str()).unwrap();
    let user = ActiveModel {
        name: Set(user_form.name.to_owned()),
        username: Set(user_form.username.to_owned()),
        password: Set(password_hashing),
        ..Default::default()
    };
    
    match user.insert(&state.database_connection).await { 
        Ok(inserted_user) => Ok((StatusCode::CREATED, api_response_single(
            vec![inserted_user].into_iter()
                .map(|user| {
                    json!({
                        "id": user.id,
                        "name": user.name,
                       "username": user.username,
                       "password": user.password,
                        "created_at": user.created_at,
                        "updated_at": user.updated_at
                   })
                }).collect::<Vec<_>>()
        ))),
        Err(err) => Err(internal_error(err))
    }
}

#[axum::debug_handler]
pub async fn get_user_credentials(
    state: State<Arc<AppState>>,
    Json(user_form): Json<UserLogin>,
) -> impl IntoResponse {
    let user = entity::user::Entity::find()
        .filter(user::Column::Username.eq(user_form.username))
        .one(&state.database_connection)
        .await;

    match user {
        Ok(Some(user_model)) => {
            match verify_password(&user_model.password, &user_form.password) { 
                Ok(true) => Ok((StatusCode::OK, api_response_single(
                    json!({
                        "id": user_model.id,
                        "name": user_model.name,
                        "username": user_model.username,
                        "profile_picture": user_model.profile_picture
                    })
                ))),
                Ok(false) => Err(internal_error("Password is incorrect")),
                Err(err) => Err(internal_error(err))
            }
        }
        
        _ => Err(not_found_error("User not found")),
    }
}