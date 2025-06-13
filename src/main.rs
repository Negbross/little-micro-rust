use crate::routes::{handle_error, routes};
use crate::utils::AppState;
use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, ConnectOptions};
use std::env;
use std::time::Duration;
use tokio::signal;
use tracing::log::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod app;
mod controllers;
mod respons;
mod routes;
mod utils;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "debug");
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenv::dotenv().ok();
    let db = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{}:{}", host, port);

    let mut opt = ConnectOptions::new(&db);
    opt.max_connections(10)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(5))
        .acquire_timeout(Duration::from_secs(5))
        .sqlx_logging(true); // logging SQLx debug

    let conn = Database::connect(opt)
        .await
        .expect("Could not connect to database");
    Migrator::up(&conn, None).await.unwrap();

    let app_state = AppState {
        database_connection: conn,
    };

    let router = routes(app_state).merge(handle_error());
    info!("Server was run by {:?}", server_url);
    let listener = tokio::net::TcpListener::bind(&server_url).await.unwrap();
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Error running server");
    info!("Server stopped");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
