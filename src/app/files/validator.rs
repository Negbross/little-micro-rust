use std::path::Path;
use slug::slugify;

/// Max chunk 2 MB
pub const MAX_CHUNK_SIZE: u64 = 2 * 1024 * 1024;
/// Max file size 50 MB
pub const MAX_UPLOAD_SIZE: u64 = 50 * 1024 * 1024;

/// Ekstensi file yang diizinkan
const ALLOWED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "mp4", "pdf"];

/// MIME type yang diizinkan berdasarkan isi file
const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "application/pdf",
    "application/epub+zip",
    "video/mp4"
];

pub fn validate_book_mime(chunk_data: &[u8]) -> Result<(), String> {
    match infer::get(chunk_data) {
        Some(kind) => {
            let mime = kind.mime_type();
            if ALLOWED_MIME_TYPES.contains(&mime) {
                Ok(())
            } else {
                Err(format!("Unknown mime type: {}", mime))
            }
        }

        None => Err(format!("Unknown mime type: {:?}", chunk_data))
    }
}

pub fn validate_chunk_size(chunk_data: &[u8], max_size_mb: usize) -> Result<(), String> {
    let max_bytes = max_size_mb * 1024 * 1024;
    if chunk_data.len() > max_bytes {
        Err(format!("File too large (max {} MB)", max_size_mb))
    } else {
        Ok(())
    }
}

/// Check if a file is safe from attackers
pub fn sanitize_filename(file_name: &str) -> String {
    let path = Path::new(file_name);

    // Get the extension
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("file")
        .to_lowercase();

    // Get without extension
    let stem = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let slug = slugify(stem);

    if ext.is_empty() {
        slug
    } else {
        format!("{}.{}", slug, ext)
    }
}

pub fn timestamped_filename(file_name: &str) -> String {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let sanitized_name = sanitize_filename(file_name);
    format!("{}_{}", timestamp, sanitized_name)
}

/// Check if a path is valid
pub fn path_is_valid(path: &str) -> bool {
    let path = Path::new(path);
    !path.components().any(|comp| matches!(
        comp,
        std::path::Component::ParentDir
        | std::path::Component::CurDir
        | std::path::Component::RootDir
    ))

    // let path = Path::new(path);
    // let mut components = path.components().peekable();
    //
    // if let Some(first) = components.peek() {
    //     if !matches!(first, std::path::Component::Normal(_)) {
    //         return false;
    //     }
    // }
    //
    // components.count() == 1
}

/// Check if upload complete
fn is_upload_complete(temp_dir: &str, total_chunks: usize) -> bool {
    match std::fs::read_dir(temp_dir) {
        Ok(entries) => entries.count() == total_chunks,
        Err(_) => false,
    }
}
