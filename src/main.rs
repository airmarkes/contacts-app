//#![allow(unused)]
pub mod models;
pub mod routers;

use crate::models::*;
use axum::Router;
use axum_login::AuthManagerLayerBuilder;
use axum_messages::MessagesManagerLayer;
use dotenv::dotenv;
use routers::*;
use sqlx::{sqlite::SqlitePool, Pool, Sqlite};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::{signal, task::AbortHandle};
use tower_http::services::ServeDir;
use tower_sessions::cookie::Key;
use tower_sessions::{session_store::ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
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
    // Generate a cryptographic key to sign the session cookie.
    let key = Key::generate();

    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(1)))
        .with_signed(key);

    // Auth service.
    //
    // This combines the session layer with our backend to establish the auth
    // service which will provide the auth session as a request extension.
    let backend = Backend::new(pool.clone());
    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    let app_state = AppState {
        pool_state: Arc::new(RwLock::new(pool)),
        contact_error_state: Arc::new(RwLock::new(CreationErrorState::default())),
        archiver_state: Arc::new(RwLock::new(ArchiverState::default())),
    };
    //let app_state = Arc::new(RwLock::new(app_state));

    let app = Router::new()
        .merge(index_router())
        .merge(show_router())
        .merge(contactform_new_router())
        .merge(view_router())
        .merge(contactform_edit_router())
        .merge(archive_router())
        .merge(utils_router())
        .merge(userform_login_router())
        .with_state(app_state)
        .layer(MessagesManagerLayer)
        .layer(auth_layer)
        .nest_service("/assets", ServeDir::new("assets"));

    //let socket = "127.0.0.1:8080";
    let socket = "0.0.0.0:8080";
    let listener = TcpListener::bind(socket).await?;
    tracing::debug!("Listening on {}\n", socket);
    println!("\nListening on {}", socket);
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
