use askama::Template;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;

use crate::archiver::ArchiverState;
use crate::errors::AppError;
use crate::{get_time, AppStateType};

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootTemplate<'a> {
    pub name: &'a str,
    pub archive_t: ArchiverState,
}

pub fn index_router() -> Router<AppStateType> {
    Router::new().route("/", get(self::get::handler_root))
}

mod get {
    use super::*;

    pub async fn handler_root(
        State(state): State<AppStateType>,
    ) -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_root", get_time());
        let root_tmpl = RootTemplate {
            name: "Guest!",
            archive_t: state.read().await.archiver_state.clone(),
        };
        Ok(root_tmpl.into_response())
    }
}
