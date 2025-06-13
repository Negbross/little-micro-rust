use regex::Regex;
use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub database_connection: DatabaseConnection,
}

pub fn slugify(text: &str) -> String {
    let re = Regex::new(r"[^\w\s-]").unwrap();
    let cleaned = re.replace_all(text, "");
    cleaned.trim()
        .to_lowercase()
        .replace(' ', "-")
        .replace("--", "-")
}
