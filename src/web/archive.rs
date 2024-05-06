use crate::errors::AppError;
use crate::functions::*;
use crate::models::*;
use askama::Template;
use axum::response::Html;
use axum::routing::get;
use axum::{extract::State, Router};
use axum::{http::header, response::IntoResponse};
use tokio_util::io::ReaderStream;

#[derive(Template)]
#[template(path = "archive_ui.html")]
pub struct ArchiveUiTemplate {
    pub archive_t: ArchiverState,
}

pub fn archive_router() -> Router<AppStateType> {
    Router::new()
        .route(
            "/contacts/archive",
            get(self::get::handler_get_archive).post(self::post::handler_post_archive),
        )
        .route(
            "/contacts/archive/file",
            get(self::get::handler_get_archive_file)
                .delete(self::delete::handler_delete_archive_file),
        )
}

mod get {
    use super::*;

    pub async fn handler_get_archive(
        State(state): State<AppStateType>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_get_archive", get_time());
        let archiver = state.read().await.archiver_state.clone();
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver,
        };
        Ok(Html(archive_ui.render()?))
    }

    pub async fn handler_get_archive_file(
        State(state): State<AppStateType>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_get_archive_file", get_time());
        let archiver = state.read().await.archiver_state.clone();
        let file = tokio::fs::File::open(archiver.archive_file()).await?;
        let stream = ReaderStream::new(file);
        let body = axum::body::Body::from_stream(stream);
        let headers = [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"contacts.db\"",
            ),
        ];
        Ok((headers, body))
    }
}

mod post {
    use super::*;

    pub async fn handler_post_archive(
        State(state): State<AppStateType>, //State(state_archive): State<ArchiverState>
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_post_archive", get_time());
        let archiver = state.read().await.archiver_state.clone();
        if archiver.archive_status == "Waiting" {
            let mut write = state.write().await;
            write.archiver_state.archive_status = "Running".to_owned();
            write.archiver_state.archive_progress = 0.0;
            drop(write);
            let clone = state.clone();
            let _handle = tokio::spawn(async move {
                run_thread(clone).await;
            });
        };
        let archiver_then = state.read().await.archiver_state.clone();

        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver_then,
        };
        Ok(Html(archive_ui.render()?))
    }
}

mod delete {
    use super::*;

    pub async fn handler_delete_archive_file(
        State(state): State<AppStateType>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_delete_archive_file", get_time());
        let mut write = state.write().await;
        write.archiver_state.archive_status = "Waiting".to_owned();
        drop(write);
        let archiver = state.read().await.archiver_state.clone();
        let archive_ui = ArchiveUiTemplate {
            archive_t: archiver,
        };
        Ok(Html(archive_ui.render()?))
    }
}
