use std::path::Path;
use crate::app::files::files::{path_storage, write_file};
use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tokio::fs;
use tracing::log::info;
use crate::app::files::validator::{ timestamped_filename, sanitize_filename };

#[axum::debug_handler]
pub async fn upload(mut payload: Multipart) -> impl IntoResponse {
    // let mut file_name = String::new();
    // let mut chunk_number = 0;
    // let mut total_chunks = 0;
    // let mut chunk_data = Vec::new();
    //
    // while let Some(field) = match payload.next_field().await {
    //     Ok(f) => f,
    //     Err(err) => {
    //         eprintln!("Error reading field: {:?}", err);
    //         return StatusCode::BAD_REQUEST;
    //     }
    // } {
    //     let field_name = field.name().unwrap().to_string();
    //     match field_name.as_str() {
    //         // "fileName" => file_name = (&field.name().unwrap_or_default()).parse().unwrap_or_default(),
    //         "fileName" => file_name = field.text().await.unwrap_or_default(),
    //         // "fileName" => file_name = name_file_date.format("%Y-%m-%d_%H-%M-%S").to_string(),
    //         "chunkNumber" => chunk_number = field.text().await.unwrap_or_default().parse().unwrap_or(0),
    //         "totalChunks" => total_chunks = field.text().await.unwrap_or_default().parse().unwrap_or(0),
    //         "chunkData" => chunk_data = field.bytes().await.unwrap_or_else(|_| Vec::new().into()).to_vec(),
    //         _ => {}
    //     }
    // }
    // info!("{}", file_name);
    //
    // if file_name.is_empty() || chunk_data.is_empty() {
    //     return StatusCode::BAD_REQUEST;
    // }
    //
    // // // File name with time chrono
    // let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    // let sanitized_name = sanitize_filename(&file_name);
    // let final_file_name = format!("{}_{}", &timestamp, sanitized_name);
    // //
    // // // Folder = uploads/<timestamp>/
    // // let folder_path = format!("uploads/{}", timestamp);
    // // let chunk_dir = path_storage(&format!("{}/chunk", folder_path));
    // // fs::create_dir_all(&chunk_dir).await.unwrap();
    //
    // let upload_dir = path_storage(&format!("uploads/{}/chunk", timestamp));
    // if let Err(err) = fs::create_dir_all(&upload_dir).await {
    //     eprintln!("Error creating directory: {:?}", err);
    //     return StatusCode::INTERNAL_SERVER_ERROR;
    // };
    //
    // let chunk_path = upload_dir.join(chunk_number.to_string());
    //
    // info!("Writing chunk {} ({} bytes)", chunk_number, chunk_data.len());
    // if let Err(err) = fs::write(&chunk_path, chunk_data).await {
    //     eprintln!("Error writing file: {:?}", err);
    //     return StatusCode::INTERNAL_SERVER_ERROR;
    // }
    //
    // if chunk_number + 1 == total_chunks {
    //     // Relative path to the storage
    //     let relative_chunk_path = format!("uploads/{}/chunk", timestamp);
    //
    //     return match write_file(&relative_chunk_path, &final_file_name, total_chunks, None).await {
    //         Ok(hash) => {
    //             info!("Successfully assembled file: {}", file_name);
    //             info!("Hash: {}", hash);
    //             StatusCode::OK
    //         },
    //         Err((status, msg)) => {
    //             eprintln!("Error combining file: {}", msg);
    //             status
    //         }
    //     }
    // }
    
    StatusCode::OK

}