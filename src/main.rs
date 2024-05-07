//#![allow(unused)]
pub mod archiver;
pub mod contacts;
pub mod errors;
pub mod users;
pub mod web;

use axum::Router;
use axum_messages::MessagesManagerLayer;
use chrono::{DateTime, Local};
use dotenv::dotenv;
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
//use std::sync::{Arc, RwLock};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::{signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

#[derive(Clone)]
pub struct AppState {
    pub contacts_state: Pool<Sqlite>,
    pub error_state: contacts::CreationErrorState,
    pub archiver_state: archiver::ArchiverState,
}
pub type AppStateType = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    let db_url: String = std::env::var("DATABASE_URL")?;
    //let db_url = "sqlite:db/contacts.db";
    let pool: Pool<Sqlite> = SqlitePool::connect(&db_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    //let session_store = MemoryStore::default();
    let session_store = SqliteStore::new(pool.clone());
    session_store.migrate().await?;

    let _deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
    );

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(time::Duration::seconds(10)));

    let app_state = AppState {
        contacts_state: pool,
        error_state: contacts::CreationErrorState::default(),
        archiver_state: archiver::ArchiverState::default(),
    };
    let app_state = Arc::new(RwLock::new(app_state));

    let app = Router::new()
        .merge(crate::web::index::index_router())
        .merge(crate::web::show::show_router())
        .merge(crate::web::new::new_router())
        .merge(crate::web::view::view_router())
        .merge(crate::web::edit::edit_router())
        .merge(crate::web::archive::archive_router())
        .merge(crate::web::utils::utils_router())
        .with_state(app_state)
        .layer(MessagesManagerLayer)
        .layer(session_layer)
        .nest_service("/assets", ServeDir::new("assets"));

    //let socket = "127.0.0.1:8080";
    let socket = "0.0.0.0:8080";
    let listener = TcpListener::bind(socket).await?;
    println!("Listening on {}\n", socket);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn _shutdown_signal(deletion_task_abort_handle: AbortHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
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
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}

pub fn get_time() -> String {
    let time_stamp_now = std::time::SystemTime::now();
    let datetime = DateTime::<Local>::from(time_stamp_now);
    let timestamp_str = datetime.format("%Y-%m-%d").to_string(); //%H:%M:%S
    timestamp_str
}
