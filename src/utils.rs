use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct AppState {
    pub database_connection: DatabaseConnection,
}
