use axum::body::Body;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use slug::slugify;
use tokio::fs;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::info;
use crate::app::files::validator::{path_is_valid, sanitize_filename};
use crate::app::hashing::hash::hash_file;

const ALLOWED_EXTENSIONS: &[&str; 6] = &["png", "jpg", "jpeg", "gif", "mp4", "pdf"];

#[derive(Deserialize)]
struct DownloadParams {
    file_name: String,
    offset: u64,
    total_chunks: usize,
}

/// Function that downloads a file using a chunk mechanism
pub fn download_file(Query(params): Query<DownloadParams>) -> impl IntoResponse {
    let file_path = path_storage(&params.file_name);
    let mut file = File::open(&file_path).unwrap_or_else(|_| {
        panic!("Failed to open file: {}", file_path.display());
    });
    let mut buffer = vec![0; params.total_chunks];
    file.seek(SeekFrom::Start(params.offset)).unwrap();
    let bytes_read = file.read(&mut buffer).unwrap();

    if bytes_read == 0 {
        return StatusCode::NO_CONTENT.into_response();
    }

    let body = Body::from(buffer[..bytes_read].to_vec());
    Response::builder()
        .header("Content-Type", "application/octet-stream")
        .header(
            "Content-Disposition",
            format!("attachment; filename={}", params.file_name),
        )
        .body(body)
        .unwrap()
}

/// Function that pointing to storage folder
pub fn path_storage(sub_path: &str) -> PathBuf {
    let base_path = std::env::current_dir().expect("Failed to get current directory");
    base_path.join("storage").join(sub_path)
}

/// Write a file as chunks
pub async fn write_file(path: &str, file_name: &str, total_chunks: usize, output_out: Option<&str>)
    -> Result<(String, String), (StatusCode, String)>
{
    if !path_is_valid(path) {
        info!("{:?}", path);
        return Err((StatusCode::NO_CONTENT, "Invalid path".to_owned()));
    }

    let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let sanitized_name = sanitize_filename(&file_name);
    let final_file_name = format!("{}_{}", &timestamp, sanitized_name);

    // let output_dir = Path::new(path).parent()
    //     .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Invalid chunk directory structure".to_string()))?; // keluar dari folder chunk
    // let output_file_name = output_dir.join(file_name).as_path().to_str().unwrap().to_string();
    let relative_path = match output_out {
        Some(subdir) => format!("uploads/{}/{}", subdir, final_file_name),
        None => format!("uploads/{}", final_file_name)
    };

    let output_path = path_storage(&relative_path);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create output dir: {}", e))
        })?;
    }


    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", e)))?;

    for chunk_number in 0..total_chunks {
        // let ck_path = format!("{}/chunk/{}", path, chunk_number);
        let ck_path = format!("{}/{}", path, chunk_number);
        let chunk_path = path_storage(&ck_path);
        let chunk_data = fs::read(&chunk_path)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read chunk {}: {}", chunk_number, e)))?;
        output_file.write_all(&chunk_data)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write chunk {}: {}", chunk_number, e)))?;
    }

    info!("{:?}", output_path);
    info!("Entering hashes mode");
    let file_bytes = fs::read(&output_path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to read file: {}", e)))?;

    let hash = hash_file(&file_bytes)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to hash file: {}", e)))?;

    info!("Entering hashes directory");
    let hash_file_path = output_path.with_extension("hash");

    fs::write(&hash_file_path, hash.as_bytes()).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write hash file: {}", e)))?;

    fs::remove_dir_all(path_storage(path))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to remove directory: {}", e)))?;
    Ok((relative_path, hash))

}
