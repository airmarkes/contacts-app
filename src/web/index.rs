use askama::Template;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;

use crate::errors::AppError;
use crate::models::*;
use crate::functions::*;

#[derive(Template)]
#[template(path = "index.html")]
pub struct RootTemplate<'a> {
    pub name: &'a str,
}

pub fn index_router() -> Router<AppStateType> {
    Router::new().route("/", get(self::get::handler_root))
}

mod get {
    use super::*;

    pub async fn handler_root() -> Result<impl IntoResponse, AppError> {
        println!("->> {} - HANDLER: handler_root", get_time());
        let root_tmpl = RootTemplate { name: "Guest!" };
        Ok(Html(root_tmpl.render()?))
    }
}
