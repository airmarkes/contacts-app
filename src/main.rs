#![allow(unused)]
pub mod errors;
pub mod models;
pub mod web;

use axum::Router;
use axum_messages::MessagesManagerLayer;
use dotenv::dotenv;
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use tokio::{signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

use crate::models::*;
use crate::web::archive::*;
use crate::web::edit::*;
use crate::web::index::*;
use crate::web::new::*;
use crate::web::show::*;
use crate::web::utils::*;
use crate::web::view::*;

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
        error_state: CreationErrorState::default(),
        archiver_state: ArchiverState::default(),
    };
    let app_state = Arc::new(RwLock::new(app_state));

    let app = Router::new()
        .merge(index_router())
        .merge(show_router())
        .merge(new_router())
        .merge(view_router())
        .merge(edit_router())
        .merge(archive_router())
        .merge(utils_router())
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

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
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
